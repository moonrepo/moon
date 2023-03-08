use crate::errors::WorkspaceError;
use moon_cache::CacheEngine;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, InheritedTasksConfig,
    InheritedTasksManager, ToolchainConfig, WorkspaceConfig,
};
use moon_constants as constants;
use moon_logger::{color, debug, trace};
use moon_platform::{BoxedPlatform, PlatformManager};
use moon_utils::{fs, glob, semver};
use moon_vcs::{Vcs, VcsLoader};
use moonbase::Moonbase;
use proto::{get_root, Config as ProtoTools, CONFIG_NAME};
use std::env;
use std::path::{Path, PathBuf};

const LOG_TARGET: &str = "moon:workspace";

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
fn find_workspace_root<P: AsRef<Path>>(current_dir: P) -> Option<PathBuf> {
    if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        let root: PathBuf = root.parse().expect("Failed to parse MOON_WORKSPACE_ROOT.");

        return Some(root);
    }

    let current_dir = current_dir.as_ref();

    trace!(
        target: "moon:workspace",
        "Attempting to find workspace root at {}",
        color::path(current_dir),
    );

    fs::find_upwards(constants::CONFIG_DIRNAME, current_dir)
        .map(|dir| dir.parent().unwrap().to_path_buf())
}

// .moon/tasks.yml, .moon/tasks/*.yml
fn load_tasks_config(root_dir: &Path) -> Result<InheritedTasksManager, WorkspaceError> {
    let mut manager = InheritedTasksManager::default();
    let old_config_path = root_dir.join(constants::CONFIG_DIRNAME).join("project.yml");
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_TASKS_FILENAME);

    // TODO: Remove in v1
    if old_config_path.exists() && !config_path.exists() {
        fs::rename(&old_config_path, &config_path)?;
    }

    let do_load = |cfg_path: &Path| match InheritedTasksConfig::load(cfg_path.to_path_buf()) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidTasksConfigFile(
            cfg_path.strip_prefix(root_dir).unwrap().to_path_buf(),
            if let ConfigError::FailedValidation(valids) = errors {
                format_figment_errors(valids)
            } else {
                format_error_line(errors.to_string())
            },
        )),
    };

    trace!(
        target: LOG_TARGET,
        "Attempting to find {} in {}",
        color::file(format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_TASKS_FILENAME,
        )),
        color::path(root_dir)
    );

    if config_path.exists() {
        manager.add_config(&config_path, do_load(&config_path)?);
    }

    trace!(
        target: LOG_TARGET,
        "Attempting to find {} in {}",
        color::file(format!("{}/{}", constants::CONFIG_DIRNAME, "tasks/*.yml")),
        color::path(root_dir)
    );

    for config_path in glob::walk_files(
        root_dir.join(constants::CONFIG_DIRNAME).join("tasks"),
        ["*.yml"],
    )? {
        trace!(target: LOG_TARGET, "Found {}", color::path(&config_path));

        manager.add_config(&config_path, do_load(&config_path)?);
    }

    Ok(manager)
}

// .moon/toolchain.yml
fn load_toolchain_config(
    root_dir: &Path,
    proto_tools: &ProtoTools,
) -> Result<ToolchainConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_TOOLCHAIN_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Loading {} from {}",
        color::file(format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_TOOLCHAIN_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Ok(ToolchainConfig::default());
    }

    match ToolchainConfig::load(config_path, proto_tools) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidToolchainConfigFile(
            if let ConfigError::FailedValidation(valids) = errors {
                format_figment_errors(valids)
            } else {
                format_error_line(errors.to_string())
            },
        )),
    }
}

// .moon/workspace.yml
fn load_workspace_config(root_dir: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_WORKSPACE_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Loading {} from {}",
        color::file(format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_WORKSPACE_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile);
    }

    match WorkspaceConfig::load(config_path) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidWorkspaceConfigFile(
            if let ConfigError::FailedValidation(valids) = errors {
                format_figment_errors(valids)
            } else {
                format_error_line(errors.to_string())
            },
        )),
    }
}

pub struct Workspace {
    /// Engine for reading and writing cache/outputs.
    pub cache: CacheEngine,

    /// Workspace configuration loaded from ".moon/workspace.yml".
    pub config: WorkspaceConfig,

    /// Registered platforms derived from toolchain configuration.
    pub platforms: PlatformManager,

    /// Proto tools loaded from ".prototools".
    pub proto_tools: ProtoTools,

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
    pub vcs: Box<dyn Vcs + Send + Sync>,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn load() -> Result<Workspace, WorkspaceError> {
        Workspace::load_from(env::current_dir().unwrap())
    }

    pub fn load_from<P: AsRef<Path>>(working_dir: P) -> Result<Workspace, WorkspaceError> {
        let working_dir = working_dir.as_ref();
        let Some(root_dir) = find_workspace_root(working_dir) else {
            return Err(WorkspaceError::MissingConfigDir);
        };

        debug!(
            target: LOG_TARGET,
            "Creating workspace at {} (from working directory {})",
            color::path(&root_dir),
            color::path(working_dir)
        );

        // Load proto tools
        let proto_path = root_dir.join(CONFIG_NAME);
        let proto_tools = if proto_path.exists() {
            ProtoTools::load(&proto_path)?
        } else {
            ProtoTools::default()
        };

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
        let vcs = VcsLoader::load(&root_dir, &config)?;

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
            vcs,
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

        let repo_slug = if self.vcs.is_enabled() {
            self.vcs.get_repository_slug().await?
        } else {
            Moonbase::no_vcs_root();

            return Ok(());
        };

        self.session = Moonbase::signin(secret_key, access_key, repo_slug).await;

        Ok(())
    }
}
