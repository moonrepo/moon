// Systems are defined in the order they should be executed!

use crate::app_error::AppError;
use moon_app_components::{AppInfo, Toolchain, WorkingDir, Workspace, WorkspaceRoot};
use moon_common::consts;
use moon_config::{ToolchainConfig, WorkspaceConfig};
use proto::{ToolsConfig, TOOLS_CONFIG_NAME};
use semver::Version;
use starbase::system;
use starbase_styles::color;
use starbase_utils::{dirs, fs};
use std::env;
use std::path::PathBuf;
use tracing::debug;

/// Detect important information about the currently running moon process.
#[system]
pub fn detect_app_process_info(resources: ResourcesMut) {
    let current_exe = env::current_exe().ok();
    let version = env!("CARGO_PKG_VERSION");

    if let Some(exe) = &current_exe {
        debug!(current_bin = ?exe.display(), "Running moon v{}", version);
    } else {
        debug!("Running moon v{}", version);
    }

    env::set_var("MOON_VERSION", version);

    resources.set(AppInfo {
        running_exe: current_exe.clone(),
        current_exe,
        global: false,
        version: Version::parse(version).unwrap(),
    });
}

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
#[system]
pub fn find_workspace_root(states: StatesMut) {
    let working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

    debug!(
        working_dir = ?working_dir.display(),
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

    // Avoid finding the ~/.moon directory
    let home_dir = dirs::home_dir().ok_or(AppError::MissingHomeDir)?;

    if home_dir == workspace_root {
        return Err(AppError::MissingConfigDir.into());
    }

    debug!(
        workspace_root = ?workspace_root.display(),
        working_dir = ?working_dir.display(),
        "Found workspace root",
    );

    states.set(WorkingDir(working_dir));
    states.set(WorkspaceRoot(workspace_root));
}

/// Load the workspace configuration file from the `.moon` directory in the workspace root.
/// This file is required to exist, so error if not found.
#[system]
pub fn load_workspace_config(workspace_root: StateRef<WorkspaceRoot>, resources: ResourcesMut) {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_WORKSPACE_FILENAME
    );
    let config_path = workspace_root.join(&config_name);

    debug!(
        file = ?config_path.display(),
        "Loading {} (required)", color::file(&config_name),
    );

    if !config_path.exists() {
        return Err(AppError::MissingConfigFile(config_name).into());
    }

    let config = WorkspaceConfig::load(&workspace_root, &config_path)?;

    resources.set(Workspace {
        telemetry: config.telemetry,
        config,
    });
}

/// Load the toolchain configuration file from the `.moon` directory if it exists.
#[system]
pub fn load_toolchain_config(workspace_root: StateRef<WorkspaceRoot>, resources: ResourcesMut) {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_TOOLCHAIN_FILENAME
    );
    let config_path = workspace_root.join(&config_name);
    let proto_path = workspace_root.join(TOOLS_CONFIG_NAME);

    debug!(
        file = ?config_path.display(),
        "Attempting to load {} (not required)", color::file(&config_name),
    );

    if proto_path.exists() {
        debug!(
            "Found a {} file in the root, loading into the toolchain",
            color::file(TOOLS_CONFIG_NAME)
        );
    }

    let proto_tools = ToolsConfig::load(proto_path)?;

    let config = if config_path.exists() {
        debug!("Config file does not exist, using defaults");

        ToolchainConfig::default()
    } else {
        ToolchainConfig::load(&workspace_root, &config_path, &proto_tools)?
    };

    resources.set(Toolchain {
        config,
        proto: proto_tools,
    });
}
