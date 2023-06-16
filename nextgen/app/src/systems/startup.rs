// Systems are defined in the order they should be executed!

use crate::app_error::AppError;
use moon_app_components::{WorkingDir, WorkspaceRoot};
use moon_common::consts;
use starbase::system;
use starbase_styles::color;
use starbase_utils::fs;
use std::env;
use std::path::PathBuf;
use tracing::debug;

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
#[system]
pub fn find_workspace_root(states: StatesMut) {
    let working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

    debug!(
        working_dir = %working_dir.display(),
        "Attempting to find workspace root from current working directory",
    );

    let workspace_root = if let Ok(root) = env::var("MOON_WORKSPACE_ROOT") {
        debug!(
            env_var = root,
            "Inheriting from {} environment variable",
            color::symbol("MOON_WORKSPACE_ROOT")
        );

        let root: PathBuf = root
            .parse()
            .map_err(|_| AppError::InvalidWorkspaceRootEnvVar)?;

        if !root.join(consts::CONFIG_DIRNAME).exists() {
            return Err(AppError::MissingConfigDir.into());
        }

        root
    } else {
        let Some(moon_dir) = fs::find_upwards(consts::CONFIG_DIRNAME, &working_dir) else {
            return Err(AppError::MissingConfigDir.into());
        };

        moon_dir.parent().unwrap().to_path_buf()
    };

    debug!(
        workspace_root = %workspace_root.display(),
        working_dir = %working_dir.display(),
        "Found workspace root",
    );

    states.set(WorkingDir(working_dir));
    states.set(WorkspaceRoot(workspace_root));
}
