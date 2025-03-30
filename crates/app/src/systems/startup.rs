use crate::app_error::AppError;
use miette::IntoDiagnostic;
use moon_common::consts::*;
use moon_config::{ConfigLoader, InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_env::MoonEnvironment;
use moon_env_var::GlobalEnvBag;
use moon_feature_flags::{FeatureFlags, Flag};
use proto_core::ProtoEnvironment;
use starbase_styles::color;
use starbase_utils::{dirs, fs};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::spawn;
use tokio::task::{JoinError, block_in_place};
use tracing::{debug, instrument};

// We need to load configuration in a blocking task, because config
// loading is synchronous but uses `reqwest::blocking` under the hood,
// which triggers a panic when used in an async context...
async fn load_config_blocking<F, R>(func: F) -> Result<R, JoinError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    spawn(async { block_in_place(func) }).await
}

/// Recursively attempt to find the workspace root by locating the ".moon"
/// configuration folder, starting from the current working directory.
#[instrument]
pub fn find_workspace_root(working_dir: &Path) -> miette::Result<PathBuf> {
    debug!(
        working_dir = ?working_dir,
        "Attempting to find workspace root from current working directory",
    );

    let workspace_root = if let Some(root) = GlobalEnvBag::instance().get("MOON_WORKSPACE_ROOT") {
        debug!(
            env_var = root,
            "Inheriting from {} environment variable",
            color::symbol("MOON_WORKSPACE_ROOT")
        );

        let root: PathBuf = root
            .parse()
            .map_err(|_| AppError::InvalidWorkspaceRootEnvVar)?;

        if !root.join(CONFIG_DIRNAME).exists() {
            return Err(AppError::MissingConfigDir.into());
        }

        root
    } else {
        fs::find_upwards_root(CONFIG_DIRNAME, working_dir).ok_or(AppError::MissingConfigDir)?
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
) -> miette::Result<Arc<MoonEnvironment>> {
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
) -> miette::Result<Arc<ProtoEnvironment>> {
    let mut env = ProtoEnvironment::new()?;
    env.working_dir = working_dir.to_path_buf();

    Ok(Arc::new(env))
}

/// Load the workspace configuration file from the `.moon` directory in the workspace root.
/// This file is required to exist, so error if not found.
#[instrument(skip(config_loader))]
pub async fn load_workspace_config(
    config_loader: ConfigLoader,
    workspace_root: &Path,
) -> miette::Result<Arc<WorkspaceConfig>> {
    let config_name = config_loader.get_debug_label("workspace", true);

    debug!("Loading {} (required)", color::file(&config_name));

    let config_files = config_loader.get_workspace_files(workspace_root);

    if config_files.iter().all(|file| !file.exists()) {
        return Err(AppError::MissingConfigFile(config_name).into());
    }

    let root = workspace_root.to_owned();
    let config = load_config_blocking(move || config_loader.load_workspace_config(root))
        .await
        .into_diagnostic()??;

    Ok(Arc::new(config))
}

/// Load the toolchain configuration file from the `.moon` directory if it exists.
#[instrument(skip(config_loader, proto_env))]
pub async fn load_toolchain_config(
    config_loader: ConfigLoader,
    proto_env: Arc<ProtoEnvironment>,
    workspace_root: &Path,
    working_dir: &Path,
) -> miette::Result<Arc<ToolchainConfig>> {
    debug!(
        "Attempting to load {} (optional)",
        color::file(config_loader.get_debug_label("toolchain", true))
    );

    let root = workspace_root.to_owned();
    let cwd = working_dir.to_owned();
    let config = load_config_blocking(move || {
        config_loader.load_toolchain_config(
            root,
            proto_env.load_config_manager()?.get_local_config(&cwd)?,
        )
    })
    .await
    .into_diagnostic()??;

    Ok(Arc::new(config))
}

/// Load the tasks configuration file from the `.moon` directory if it exists.
/// Also load all scoped tasks from the `.moon/tasks` directory and load into the manager.
#[instrument(skip(config_loader))]
pub async fn load_tasks_configs(
    config_loader: ConfigLoader,
    workspace_root: &Path,
) -> miette::Result<Arc<InheritedTasksManager>> {
    debug!(
        "Attempting to load {} and {} (optional)",
        color::file(config_loader.get_debug_label("tasks", true)),
        color::file(config_loader.get_debug_label("tasks/**/*", true)),
    );

    let root = workspace_root.to_owned();
    let manager = load_config_blocking(move || config_loader.load_tasks_manager(root))
        .await
        .into_diagnostic()??;

    debug!(
        scopes = ?manager.configs.keys(),
        "Loaded {} task configs to inherit",
        manager.configs.len(),
    );

    Ok(Arc::new(manager))
}

#[instrument(skip_all)]
pub fn register_feature_flags(config: &WorkspaceConfig) -> miette::Result<()> {
    FeatureFlags::default()
        .set(Flag::FastGlobWalk, config.experiments.faster_glob_walk)
        .set(Flag::GitV2, config.experiments.git_v2)
        .register();

    Ok(())
}

#[instrument(skip_all)]
pub fn create_moonx_shims() -> miette::Result<()> {
    let Ok(exe_file) = env::current_exe() else {
        return Ok(());
    };

    let shim_file =
        exe_file
            .parent()
            .unwrap()
            .join(if cfg!(windows) { "moonx.ps1" } else { "moonx" });

    if shim_file.exists() {
        return Ok(());
    }

    match fs::write_file(&shim_file, get_moonx_shim_content()) {
        Ok(_) => {
            if let Err(error) = fs::update_perms(&shim_file, None) {
                debug!("Failed to make moonx shim executable: {error}");

                let _ = fs::remove_file(shim_file);
            }
        }
        Err(error) => {
            debug!("Failed to create moonx shim: {error}");
        }
    }

    Ok(())
}

#[cfg(unix)]
fn get_moonx_shim_content() -> String {
    r#"#!/usr/bin/env sh

exec moon run "$@"
exit $?
"#
    .into()
}

#[cfg(windows)]
fn get_moonx_shim_content() -> String {
    r#"#!/usr/bin/env pwsh

if ($MyInvocation.ExpectingInput) {
  $input | & moon run $args
} else {
  & moon run $args
}

exit $LASTEXITCODE
"#
    .into()
}
