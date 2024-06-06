// Systems are defined in the order they should be executed!

use crate::app_error::AppError;
use moon_common::consts;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_env::MoonEnvironment;
use proto_core::{ProtoConfig, ProtoEnvironment};
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{dirs, fs};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

// /// Detect important information about the currently running moon process.
// #[system]
// pub fn detect_app_process_info(resources: ResourcesMut) {
//     let current_exe = env::current_exe().ok();
//     let version = env!("CARGO_PKG_VERSION");

//     if let Some(exe) = &current_exe {
//         debug!(current_bin = ?exe, "Running moon v{}", version);
//     } else {
//         debug!("Running moon v{}", version);
//     }

//     env::set_var("MOON_VERSION", version);

//     resources.set(AppInfo {
//         running_exe: current_exe.clone(),
//         current_exe,
//         global: false,
//         version: Version::parse(version).unwrap(),
//     });
// }

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
#[instrument]
pub fn find_workspace_root(working_dir: &Path) -> AppResult<PathBuf> {
    debug!(
        working_dir = ?working_dir,
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
        fs::find_upwards_root(consts::CONFIG_DIRNAME, &working_dir)
            .ok_or(AppError::MissingConfigDir)?
    };

    // Avoid finding the ~/.moon directory
    let home_dir = dirs::home_dir().ok_or(AppError::MissingHomeDir)?;

    if home_dir == workspace_root {
        return Err(AppError::MissingConfigDir.into());
    }

    debug!(
        workspace_root = ?workspace_root,
        working_dir = ?working_dir,
        "Found workspace root",
    );

    Ok(workspace_root)
}

/// Detect information for moon from the environment.
#[instrument]
pub fn detect_moon_environment(
    working_dir: &Path,
    workspace_root: &Path,
) -> AppResult<Arc<MoonEnvironment>> {
    let mut env = MoonEnvironment::new()?;
    env.working_dir = working_dir.to_path_buf();
    env.workspace_root = workspace_root.to_path_buf();

    Ok(Arc::new(env))
}

/// Detect information for proto from the environment.
#[instrument]
pub fn detect_proto_environment(
    working_dir: &Path,
    _workspace_root: &Path,
) -> AppResult<Arc<ProtoEnvironment>> {
    let mut env = ProtoEnvironment::new()?;
    env.cwd = working_dir.to_path_buf();
    // env.workspace_root = workspace_root.to_path_buf();

    Ok(Arc::new(env))
}

/// Load the workspace configuration file from the `.moon` directory in the workspace root.
/// This file is required to exist, so error if not found.
#[instrument]
pub fn load_workspace_config(workspace_root: &Path) -> AppResult<WorkspaceConfig> {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_WORKSPACE_FILENAME
    );
    let config_file = workspace_root.join(&config_name);

    debug!(
        config_file = ?config_file,
        "Loading {} (required)", color::file(&config_name),
    );

    if !config_file.exists() {
        return Err(AppError::MissingConfigFile(config_name).into());
    }

    let config = WorkspaceConfig::load(workspace_root, &config_file)?;

    Ok(config)
}

/// Load the toolchain configuration file from the `.moon` directory if it exists.
#[instrument]
pub fn load_toolchain_config(
    workspace_root: &Path,
    proto_config: &ProtoConfig,
) -> AppResult<ToolchainConfig> {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_TOOLCHAIN_FILENAME
    );
    let config_file = workspace_root.join(&config_name);

    debug!(
        config_file = ?config_file,
        "Attempting to load {} (optional)",
        color::file(config_name),
    );

    let config = if config_file.exists() {
        debug!("Config file does not exist, using defaults");

        ToolchainConfig::default()
    } else {
        ToolchainConfig::load(workspace_root, &config_file, proto_config)?
    };

    Ok(config)
}

/// Load the tasks configuration file from the `.moon` directory if it exists.
/// Also load all scoped tasks from the `.moon/tasks` directory and load into the manager.
#[instrument]
pub fn load_tasks_configs(workspace_root: &Path) -> AppResult<Arc<InheritedTasksManager>> {
    let config_name = format!(
        "{}/{}",
        consts::CONFIG_DIRNAME,
        consts::CONFIG_TASKS_FILENAME
    );
    let config_file = workspace_root.join(&config_name);

    debug!(
        config_file = ?config_file,
        "Attempting to load {} and {} (optional)",
        color::file(config_name),
        color::file(format!("{}/tasks/**/*.yml", consts::CONFIG_DIRNAME)),
    );

    let manager = InheritedTasksManager::load_from(workspace_root)?;

    debug!(
        scopes = ?manager.configs.keys(),
        "Loaded {} task configs to inherit",
        manager.configs.len(),
    );

    Ok(Arc::new(manager))
}
