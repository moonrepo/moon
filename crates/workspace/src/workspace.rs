use crate::errors::WorkspaceError;
use moon_cache::CacheEngine;
use moon_config::{
    format_error_line, format_figment_errors, ConfigError, GlobalProjectConfig, WorkspaceConfig,
};
use moon_constants as constants;
use moon_logger::{color, debug, trace};
use moon_project_graph::ProjectGraph;
use moon_toolchain::Toolchain;
use moon_utils::fs;
use moon_vcs::{Vcs, VcsLoader};
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
        color::path(&current_dir),
    );

    fs::find_upwards(constants::CONFIG_DIRNAME, &current_dir)
        .map(|dir| dir.parent().unwrap().to_path_buf())
}

// .moon/project.yml
async fn load_global_project_config(
    root_dir: &Path,
) -> Result<GlobalProjectConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_GLOBAL_PROJECT_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Attempting to find {} in {}",
        color::file(&format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_GLOBAL_PROJECT_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Ok(GlobalProjectConfig::default());
    }

    match GlobalProjectConfig::load(config_path).await {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(WorkspaceError::InvalidGlobalProjectConfigFile(
            if let ConfigError::FailedValidation(valids) = errors {
                format_figment_errors(valids)
            } else {
                format_error_line(errors.to_string())
            },
        )),
    }
}

// .moon/workspace.yml
async fn load_workspace_config(root_dir: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
    let config_path = root_dir
        .join(constants::CONFIG_DIRNAME)
        .join(constants::CONFIG_WORKSPACE_FILENAME);

    trace!(
        target: LOG_TARGET,
        "Loading {} from {}",
        color::file(&format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_WORKSPACE_FILENAME,
        )),
        color::path(root_dir)
    );

    if !config_path.exists() {
        return Err(WorkspaceError::MissingWorkspaceConfigFile);
    }

    match WorkspaceConfig::load(config_path).await {
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

    /// The project graph, where each project is lazy loaded in.
    pub projects: ProjectGraph,

    /// The root of the workspace that contains the ".moon" config folder.
    pub root: PathBuf,

    /// The toolchain instance that houses all runtime tools/languages.
    pub toolchain: Toolchain,

    /// Configured version control system.
    pub vcs: Box<dyn Vcs + Send + Sync>,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub async fn load() -> Result<Workspace, WorkspaceError> {
        Workspace::load_from(env::current_dir().unwrap()).await
    }

    pub async fn load_from<P: AsRef<Path>>(working_dir: P) -> Result<Workspace, WorkspaceError> {
        let working_dir = working_dir.as_ref();
        let root_dir = match find_workspace_root(working_dir) {
            Some(dir) => dir,
            None => return Err(WorkspaceError::MissingConfigDir),
        };

        debug!(
            target: LOG_TARGET,
            "Creating workspace at {} (from working directory {})",
            color::path(&root_dir),
            color::path(&working_dir)
        );

        // Load configs
        let config = load_workspace_config(&root_dir).await?;
        let project_config = load_global_project_config(&root_dir).await?;

        // Setup components
        let cache = CacheEngine::load(&root_dir).await?;
        let toolchain = Toolchain::load(&config).await?;
        let projects = ProjectGraph::generate(&root_dir, &config, project_config, &cache).await?;
        let vcs = VcsLoader::load(&root_dir, &config)?;

        Ok(Workspace {
            cache,
            config,
            projects,
            root: root_dir,
            toolchain,
            vcs,
            working_dir: working_dir.to_owned(),
        })
    }
}
