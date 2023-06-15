use crate::errors::WorkspaceError;
use moon_cache::CacheEngine;
use moon_common::consts;
use moon_config::{InheritedTasksConfig, InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_error::MoonError;
use moon_logger::{debug, trace};
use moon_platform::{BoxedPlatform, PlatformManager};
use moon_utils::semver;
use moon_vcs2::{BoxedVcs, Git};
use moonbase::Moonbase;
use proto::{get_root, ToolsConfig, TOOLS_CONFIG_NAME};
use starbase_styles::color;
use starbase_utils::{dirs, fs, glob};
use std::env;
use std::path::{Path, PathBuf};

const LOG_TARGET: &str = "moon:workspace";

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
fn find_workspace_root<P: AsRef<Path>>(current_dir: P) -> Result<PathBuf, WorkspaceError> {
    if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        let root: PathBuf = root.parse().expect("Failed to parse MOON_WORKSPACE_ROOT.");

        return Ok(root);
    }

    let current_dir = current_dir.as_ref();

    trace!(
        target: "moon:workspace",
        "Attempting to find workspace root at {}",
        color::path(current_dir),
    );

    let Some(possible_root) = fs::find_upwards(consts::CONFIG_DIRNAME, current_dir)
        .map(|dir| dir.parent().unwrap().to_path_buf()) else {
        return Err(WorkspaceError::MissingConfigDir);
    };

    // Avoid finding the ~/.moon directory
    let home_dir = dirs::home_dir().ok_or(WorkspaceError::MissingHomeDir)?;

    if home_dir == possible_root {
        return Err(WorkspaceError::MissingConfigDir);
    }

    Ok(possible_root)
}

// .moon/tasks.yml, .moon/tasks/*.yml
fn load_tasks_config(root_dir: &Path) -> Result<InheritedTasksManager, WorkspaceError> {
    let mut manager = InheritedTasksManager::default();
    let config_path = root_dir
        .join(consts::CONFIG_DIRNAME)
        .join(consts::CONFIG_TASKS_FILENAME);

    let do_load = |cfg_path: &Path| InheritedTasksConfig::load_partial(root_dir, cfg_path);

    trace!(
        target: LOG_TARGET,
        "Attempting to find {} in {}",
        color::file(format!(
            "{}/{}",
            consts::CONFIG_DIRNAME,
            consts::CONFIG_TASKS_FILENAME,
        )),
        color::path(root_dir)
    );

    if config_path.exists() {
        manager.add_config(&config_path, do_load(&config_path)?);
    }

    trace!(
        target: LOG_TARGET,
        "Attempting to find {} in {}",
        color::file(format!("{}/{}", consts::CONFIG_DIRNAME, "tasks/*.yml")),
        color::path(root_dir)
    );

    for config_path in glob::walk_files(
        root_dir.join(consts::CONFIG_DIRNAME).join("tasks"),
        ["*.yml"],
    )
    .map_err(MoonError::StarGlob)?
    {
        trace!(target: LOG_TARGET, "Found {}", color::path(&config_path));

        manager.add_config(&config_path, do_load(&config_path)?);
    }

    Ok(manager)
}

// .moon/toolchain.yml
fn load_toolchain_config(
    root_dir: &Path,
    proto_tools: &ToolsConfig,
) -> Result<ToolchainConfig, WorkspaceError> {
    let config_path = root_dir
        .join(consts::CONFIG_DIRNAME)
        .join(consts::CONFIG_TOOLCHAIN_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Loading {} from {}",
        color::file(format!(
            "{}/{}",
            consts::CONFIG_DIRNAME,
            consts::CONFIG_TOOLCHAIN_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Ok(ToolchainConfig::default());
    }

    Ok(ToolchainConfig::load_from(root_dir, proto_tools)?)
}

// .moon/workspace.yml
fn load_workspace_config(root_dir: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
    let config_path = root_dir
        .join(consts::CONFIG_DIRNAME)
        .join(consts::CONFIG_WORKSPACE_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Loading {} from {}",
        color::file(format!(
            "{}/{}",
            consts::CONFIG_DIRNAME,
            consts::CONFIG_WORKSPACE_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile);
    }

    Ok(WorkspaceConfig::load_from(root_dir)?)
}

pub struct Workspace {
    /// Engine for reading and writing cache/outputs.
    pub cache: CacheEngine,

    /// Workspace configuration loaded from ".moon/workspace.yml".
    pub config: WorkspaceConfig,

    /// Registered platforms derived from toolchain configuration.
    pub platforms: PlatformManager,

    /// Proto tools loaded from ".prototools".
    pub proto_tools: ToolsConfig,

    /// The root of the workspace that contains the ".moon" config folder.
    pub root: PathBuf,

    /// When logged in, the auth token and IDs for making API requests.
    pub session: Option<Moonbase>,

    /// Global tasks configuration loaded from ".moon/tasks.yml".
    pub tasks_config: InheritedTasksManager,

    /// Toolchain configuration loaded from ".moon/toolchain.yml".
    pub toolchain_config: ToolchainConfig,

    /// The root of the toolchain, typically "~/.proto".
    pub toolchain_root: PathBuf,

    /// Configured version control system.
    pub vcs: BoxedVcs,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn load_from<P: AsRef<Path>>(working_dir: P) -> Result<Workspace, WorkspaceError> {
        let working_dir = working_dir.as_ref();
        let root_dir = find_workspace_root(working_dir)?;

        debug!(
            target: LOG_TARGET,
            "Creating workspace at {} (from working directory {})",
            color::path(&root_dir),
            color::path(working_dir)
        );

        // Load proto tools
        let proto_tools = ToolsConfig::load(root_dir.join(TOOLS_CONFIG_NAME))?;

        // Load configs
        let config = load_workspace_config(&root_dir)?;
        let toolchain_config = load_toolchain_config(&root_dir, &proto_tools)?;
        let tasks_config = load_tasks_config(&root_dir)?;

        if let Some(constraint) = &config.version_constraint {
            if let Ok(current_version) = env::var("MOON_VERSION") {
                if !semver::satisfies_range(&current_version, constraint) {
                    return Err(WorkspaceError::InvalidMoonVersion(
                        current_version,
                        constraint.to_owned(),
                    ));
                }
            }
        }

        // Setup components
        let cache = CacheEngine::load(&root_dir)?;
        let vcs = Git::load(
            &root_dir,
            &config.vcs.default_branch,
            &config.vcs.remote_candidates,
        )?;

        Ok(Workspace {
            cache,
            config,
            platforms: PlatformManager::default(),
            proto_tools,
            root: root_dir,
            session: None,
            tasks_config,
            toolchain_config,
            toolchain_root: get_root()?,
            vcs: Box::new(vcs),
            working_dir: working_dir.to_owned(),
        })
    }

    pub fn register_platform(&mut self, platform: BoxedPlatform) {
        self.platforms.register(platform.get_type(), platform);
    }

    pub async fn signin_to_moonbase(&mut self) -> Result<(), WorkspaceError> {
        let Ok(secret_key) = env::var("MOONBASE_SECRET_KEY") else {
            return Ok(());
        };

        let Ok(access_key) = env::var("MOONBASE_ACCESS_KEY")
            .or_else(|_| env::var("MOONBASE_API_KEY")) else {
            return Ok(());
        };

        let Ok(repo_slug) = env::var("MOONBASE_REPO_SLUG")
            .or_else(|_| env::var("MOON_REPO_SLUG")) else {
            Moonbase::no_vcs_root();

            return Ok(());
        };

        self.session = Moonbase::signin(secret_key, access_key, repo_slug).await;

        Ok(())
    }
}
