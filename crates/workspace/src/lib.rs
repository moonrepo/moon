mod errors;

use errors::WorkspaceError;
use monolith_config::{constants, GlobalProjectConfig, WorkspaceConfig};
use monolith_toolchain::Toolchain;
use std::env;
use std::path::{Path, PathBuf};

/// Recursively attempt to find the workspace root by locating the ".monolith"
/// configuration folder, starting from the current working directory.
fn find_workspace_root(current_dir: PathBuf) -> Option<PathBuf> {
    let config_dir = current_dir.join(constants::CONFIG_DIRNAME);

    if config_dir.exists() {
        return Some(current_dir);
    }

    let parent_dir = current_dir.parent();

    match parent_dir {
        Some(dir) => find_workspace_root(dir.to_path_buf()),
        None => None,
    }
}

#[derive(Debug)]
pub struct Workspace {
    /// Workspace configuration loaded from ".monolith/workspace.yml".
    pub config: WorkspaceConfig,

    /// Global project configuration loaded from ".monolith/project.yml".
    pub project_config: GlobalProjectConfig,

    /// The root of the workspace that contains the ".monolith" config folder.
    pub root_dir: PathBuf,

    /// The toolchain instances that houses all runtime tools/languages.
    pub toolchain: Toolchain,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    // project.yml
    fn load_global_project_config(root_dir: &Path) -> Result<GlobalProjectConfig, WorkspaceError> {
        let config_path = root_dir
            .join(constants::CONFIG_DIRNAME)
            .join(constants::CONFIG_PROJECT_FILENAME);

        if !config_path.exists() {
            return Err(WorkspaceError::MissingGlobalProjectConfigFile);
        }

        match GlobalProjectConfig::load(config_path) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(WorkspaceError::InvalidGlobalProjectConfigFile(errors)),
        }
    }

    // workspace.yml
    fn load_workspace_config(root_dir: &Path) -> Result<WorkspaceConfig, WorkspaceError> {
        let config_path = root_dir
            .join(constants::CONFIG_DIRNAME)
            .join(constants::CONFIG_WORKSPACE_FILENAME);

        if !config_path.exists() {
            return Err(WorkspaceError::MissingWorkspaceConfigFile);
        }

        match WorkspaceConfig::load(config_path) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(WorkspaceError::InvalidWorkspaceConfigFile(errors)),
        }
    }

    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn load() -> Result<Workspace, WorkspaceError> {
        let working_dir = env::current_dir().unwrap();

        // Find root dir
        let root_dir = match find_workspace_root(working_dir.clone()) {
            Some(dir) => dir,
            None => return Err(WorkspaceError::MissingConfigDir),
        };

        // Load configs
        let config = Workspace::load_workspace_config(&root_dir)?;
        let project_config = Workspace::load_global_project_config(&root_dir)?;

        // Setup toolchain
        let toolchain = Toolchain::load(&config)?;

        Ok(Workspace {
            config,
            project_config,
            root_dir,
            toolchain,
            working_dir,
        })
    }
}
