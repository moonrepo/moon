use crate::workspace_error::WorkspaceError;
use moon_api::Moonbase;
use moon_cache::CacheEngine;
use moon_common::consts;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_hash::HashEngine;
use moon_vcs::{BoxedVcs, Git};
use proto_core::{ProtoConfig, ProtoEnvironment, Version};
use starbase::Resource;
use starbase_styles::color;
use starbase_utils::{dirs, fs};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::debug;

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
fn find_workspace_root<P: AsRef<Path>>(working_dir: P) -> miette::Result<PathBuf> {
    if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        let root: PathBuf = root.parse().expect("Failed to parse MOON_WORKSPACE_ROOT.");

        return Ok(root);
    }

    let working_dir = working_dir.as_ref();
    let home_dir = dirs::home_dir().unwrap();

    debug!(
        working_dir = ?working_dir,
        "Attempting to find workspace root",
    );

    let Some(possible_root) =
        fs::find_upwards_root_until(consts::CONFIG_DIRNAME, working_dir, &home_dir)
    else {
        return Err(WorkspaceError::MissingConfigDir.into());
    };

    // Avoid finding the ~/.moon directory
    if home_dir == possible_root {
        return Err(WorkspaceError::MissingConfigDir.into());
    }

    debug!(
        workspace_root = ?possible_root,
        "Found a potential workspace root",
    );

    Ok(possible_root)
}

// .moon/tasks.yml, .moon/tasks/**/*.yml
fn load_tasks_config(root_dir: &Path) -> miette::Result<InheritedTasksManager> {
    debug!(
        workspace_root = ?root_dir,
        "Attempting to load {}",
        color::file(format!(
            "{}/{}",
            consts::CONFIG_DIRNAME,
            consts::CONFIG_TASKS_FILENAME,
        )),
    );

    debug!(
        workspace_root = ?root_dir,
        "Attempting to load {}",
        color::file(format!("{}/{}", consts::CONFIG_DIRNAME, "tasks/**/*.yml")),
    );

    let manager = InheritedTasksManager::load_from(root_dir)?;

    debug!(
        scopes = ?manager.configs.keys(),
        "Loaded {} task configs to inherit",
        manager.configs.len(),
    );

    Ok(manager)
}

// .moon/toolchain.yml
fn load_toolchain_config(
    root_dir: &Path,
    proto_config: &ProtoConfig,
) -> miette::Result<ToolchainConfig> {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_TOOLCHAIN_FILENAME,
    );
    let config_path = root_dir.join(&config_name);

    debug!(
        workspace_root = ?root_dir,
        "Attempting to load {}",
        color::file(config_name),
    );

    if !config_path.exists() {
        return Ok(ToolchainConfig::default());
    }

    ToolchainConfig::load_from(root_dir, proto_config)
}

// .moon/workspace.yml
fn load_workspace_config(root_dir: &Path) -> miette::Result<WorkspaceConfig> {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_WORKSPACE_FILENAME,
    );
    let config_path = root_dir.join(&config_name);

    debug!(
        workspace_root = ?root_dir,
        "Loading {}",
        color::file(config_name),
    );

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile.into());
    }

    WorkspaceConfig::load_from(root_dir)
}

#[derive(Clone, Resource)]
pub struct Workspace {
    /// Engine for reading and writing cache/states.
    pub cache_engine: Arc<CacheEngine>,

    /// Workspace configuration loaded from ".moon/workspace.yml".
    pub config: Arc<WorkspaceConfig>,

    /// Engine for reading and writing hashes/outputs.
    pub hash_engine: Arc<HashEngine>,

    /// Local `.prototools` config.
    pub proto_config: Arc<ProtoConfig>,

    /// The root of the workspace that contains the ".moon" config folder.
    pub root: PathBuf,

    /// When logged in, the auth token and IDs for making API requests.
    pub session: Option<Arc<Moonbase>>,

    /// Global tasks configuration loaded from ".moon/tasks.yml".
    pub tasks_config: Arc<InheritedTasksManager>,

    /// Toolchain configuration loaded from ".moon/toolchain.yml".
    pub toolchain_config: Arc<ToolchainConfig>,

    /// Configured version control system.
    pub vcs: Arc<BoxedVcs>,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn load_from<P: AsRef<Path>, E: AsRef<ProtoEnvironment>>(
        working_dir: P,
        proto_env: E,
    ) -> miette::Result<Workspace> {
        let working_dir = working_dir.as_ref();
        let root_dir = find_workspace_root(working_dir)?;

        debug!(
            workspace_root = ?root_dir,
            working_dir = ?working_dir,
            "Creating workspace",
        );

        // Load configs
        let config = load_workspace_config(&root_dir)?;
        let proto_config = proto_env
            .as_ref()
            .load_config_manager()?
            .get_local_config(working_dir)?;
        let toolchain_config = load_toolchain_config(&root_dir, proto_config)?;
        let tasks_config = load_tasks_config(&root_dir)?;

        if let Some(constraint) = &config.version_constraint {
            if let Ok(current_version) = env::var("MOON_VERSION") {
                let version = Version::parse(&current_version);

                if version.is_err() || !constraint.matches(&version.unwrap()) {
                    return Err(WorkspaceError::InvalidMoonVersion {
                        actual: current_version,
                        expected: constraint.to_string(),
                    }
                    .into());
                }
            }
        }

        // Setup components
        let cache_engine = CacheEngine::new(&root_dir)?;
        let hash_engine = HashEngine::new(&cache_engine.cache_dir)?;
        let vcs = Git::load(
            &root_dir,
            &config.vcs.default_branch,
            &config.vcs.remote_candidates,
        )?;

        Ok(Workspace {
            cache_engine: Arc::new(cache_engine),
            config: Arc::new(config),
            hash_engine: Arc::new(hash_engine),
            proto_config: Arc::new(proto_config.to_owned()),
            root: root_dir,
            session: None,
            tasks_config: Arc::new(tasks_config),
            toolchain_config: Arc::new(toolchain_config),
            vcs: Arc::new(Box::new(vcs)),
            working_dir: working_dir.to_owned(),
        })
    }

    pub async fn signin_to_moonbase(&mut self) -> miette::Result<()> {
        let Ok(secret_key) = env::var("MOONBASE_SECRET_KEY") else {
            return Ok(());
        };

        let Ok(repo_slug) = env::var("MOONBASE_REPO_SLUG") else {
            Moonbase::no_vcs_root();

            return Ok(());
        };

        self.session = Moonbase::signin(secret_key, repo_slug).await.map(Arc::new);

        Ok(())
    }
}
