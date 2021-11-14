mod errors;

use errors::WorkspaceError;
use monolith_config::{constants, WorkspaceConfig};
use std::env;
use std::path::PathBuf;

/// Recursively attempt to find the workspace root by locating the ".monolith"
/// configuration folder, starting from the current working directory.
fn find_workspace_root(current_dir: PathBuf) -> Option<PathBuf> {
    let config_dir = current_dir.clone().join(constants::CONFIG_DIRNAME);

    if config_dir.exists() {
        return Some(current_dir);
    }

    let parent_dir = current_dir.parent();

    match parent_dir {
        Some(dir) => find_workspace_root(dir.to_path_buf()),
        None => None,
    }
}

pub struct Workspace {
    pub config: WorkspaceConfig,

    /// The root of the workspace that contains the ".monolith" config folder.
    pub root_dir: PathBuf,

    /// The current working directory.
    pub working_dir: PathBuf,
}

impl Workspace {
    /// Create a new workspace instance starting from the current working directory.
    /// Will locate the workspace root and load available configuration files.
    pub fn new() -> Result<Workspace, WorkspaceError> {
        let working_dir = env::current_dir().unwrap();

        // Find root dir
        let root_dir = match find_workspace_root(working_dir.clone()) {
            Some(dir) => dir,
            None => return Err(WorkspaceError::MissingConfigDir),
        };

        // Load "workspace.yml"
        let config_path = root_dir
            .clone()
            .join(constants::CONFIG_DIRNAME)
            .join(constants::CONFIG_WORKSPACE_FILENAME);

        if !config_path.exists() {
            return Err(WorkspaceError::MissingWorkspaceConfigFile);
        }

        let config = match WorkspaceConfig::load(config_path) {
            Ok(cfg) => cfg,
            Err(errors) => return Err(WorkspaceError::InvalidWorkspaceConfigFile(errors)),
        };

        Ok(Workspace {
            config,
            root_dir,
            working_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
