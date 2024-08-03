use crate::app_error::AppError;
use miette::IntoDiagnostic;
use moon_api::Moonbase;
use moon_common::{consts::*, get_config_file_label};
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_env::MoonEnvironment;
use moon_vcs::BoxedVcs;
use proto_core::ProtoEnvironment;
use starbase::AppResult;
use starbase_styles::color;
use starbase_utils::{dirs, fs};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::spawn;
use tokio::task::{block_in_place, JoinError};
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
pub async fn load_workspace_config(workspace_root: &Path) -> AppResult<Arc<WorkspaceConfig>> {
    let config_dir = workspace_root.join(CONFIG_DIRNAME);
    let config_name = get_config_file_label("workspace", true);

    debug!("Loading {} (required)", color::file(&config_name),);

    let pkl_file = config_dir.join(CONFIG_WORKSPACE_FILENAME_PKL);
    let yml_file = config_dir.join(CONFIG_WORKSPACE_FILENAME_YML);

    if !pkl_file.exists() && !yml_file.exists() {
        return Err(AppError::MissingConfigFile(config_name).into());
    }

    let root = workspace_root.to_owned();
    let config = load_config_blocking(move || WorkspaceConfig::load_from(root))
        .await
        .into_diagnostic()??;

    Ok(Arc::new(config))
}

/// Load the toolchain configuration file from the `.moon` directory if it exists.
#[instrument(skip(proto_env))]
pub async fn load_toolchain_config(
    workspace_root: &Path,
    working_dir: &Path,
    proto_env: Arc<ProtoEnvironment>,
) -> AppResult<Arc<ToolchainConfig>> {
    let config_name = get_config_file_label("toolchain", true);

    debug!("Attempting to load {} (optional)", color::file(config_name));

    let root = workspace_root.to_owned();
    let cwd = working_dir.to_owned();
    let config = load_config_blocking(move || {
        ToolchainConfig::load_from(
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
#[instrument]
pub async fn load_tasks_configs(workspace_root: &Path) -> AppResult<Arc<InheritedTasksManager>> {
    let config_name = format!("{}/{}", CONFIG_DIRNAME, CONFIG_TASKS_FILENAME);
    let config_file = workspace_root.join(&config_name);

    debug!(
        config_file = ?config_file,
        "Attempting to load {} and {} (optional)",
        color::file(config_name),
        color::file(format!("{}/tasks/**/*", CONFIG_DIRNAME)),
    );

    let root = workspace_root.to_owned();
    let manager = load_config_blocking(move || InheritedTasksManager::load_from(root))
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
pub async fn signin_to_moonbase(vcs: &BoxedVcs) -> AppResult<Option<Arc<Moonbase>>> {
    if vcs.is_enabled() && env::var("MOONBASE_REPO_SLUG").is_err() {
        if let Ok(slug) = vcs.get_repository_slug().await {
            env::set_var("MOONBASE_REPO_SLUG", slug.as_str());
        }
    }

    let Ok(secret_key) = env::var("MOONBASE_SECRET_KEY") else {
        return Ok(None);
    };

    let Ok(repo_slug) = env::var("MOONBASE_REPO_SLUG") else {
        Moonbase::no_vcs_root();

        return Ok(None);
    };

    Ok(Moonbase::signin(secret_key, repo_slug).await)
}
