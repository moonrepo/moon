use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_error::map_io_to_fs_error;
use moon_hasher::HashSet;
use moon_logger::{color, debug, warn};
use moon_platform::Runtime;
use moon_project::Project;
use moon_utils::{fs, is_offline, time};
use moon_workspace::Workspace;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:install-deps";

// We need to include the runtime and project in the key,
// since moon can install deps in multiple projects across multiple tools
// all in parallel!
fn get_installation_key(runtime: &Runtime, project: Option<&Project>) -> String {
    format!(
        "{}:{}",
        runtime,
        match project {
            Some(p) => p.id.as_ref(),
            None => "*",
        }
    )
}

pub async fn install_deps(
    _action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
    project: Option<&Project>,
) -> Result<ActionStatus, PipelineError> {
    env::set_var("MOON_RUNNING_ACTION", "install-deps");

    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    let workspace = workspace.read().await;
    let context = context.read().await;
    let install_key = get_installation_key(runtime, project);

    if is_offline() {
        warn!(
            target: LOG_TARGET,
            "No internet connection, assuming offline and skipping install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When the install is happening as a child process of another install, avoid recursion
    if env::var("MOON_INSTALLING_DEPS").unwrap_or_default() == install_key {
        debug!(
            target: LOG_TARGET,
            "Detected another install running, skipping install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if workspace.cache.get_mode().is_write_only() {
        debug!(target: LOG_TARGET, "Force updating cache, skipping install");

        return Ok(ActionStatus::Skipped);
    }

    // When running against affected files, avoid install as it interrupts the workflow
    if context.affected_only {
        debug!(
            target: LOG_TARGET,
            "Running against affected files, skipping install",
        );

        return Ok(ActionStatus::Skipped);
    }

    let platform = workspace.platforms.get(runtime)?;

    let Some((lockfile, manifest)) = platform.get_dependency_configs()? else {
        debug!(
            target: LOG_TARGET,
            "No dependency manager for language, skipping install",
        );

        return Ok(ActionStatus::Skipped);
    };

    // Determine the working directory and whether lockfiles and manifests have been modified
    let working_dir = project.map(|p| &p.root).unwrap_or_else(|| &workspace.root);
    let manifest_path = working_dir.join(&manifest);
    let lockfile_path = working_dir.join(&lockfile);
    let mut hashset = HashSet::default();
    let mut last_modified = 0;

    if manifest_path.exists() {
        platform
            .hash_manifest_deps(&manifest_path, &mut hashset, &workspace.config.hasher)
            .await?;
    }

    if lockfile_path.exists() {
        last_modified = time::to_millis(
            fs::metadata(&lockfile_path)?
                .modified()
                .map_err(|e| map_io_to_fs_error(e, lockfile_path.clone()))?,
        );
    }

    // When running in the workspace root, account for nested manifests
    if project.is_none() {
        for touched_file in &context.touched_files {
            if touched_file.ends_with(&manifest) && touched_file != &manifest_path {
                platform
                    .hash_manifest_deps(touched_file, &mut hashset, &workspace.config.hasher)
                    .await?;
            }
        }
    }

    // Install dependencies in the working directory
    let hash = hashset.generate();
    let mut cache = workspace
        .cache
        .cache_deps_state(runtime, project.map(|p| p.id.as_ref()))?;

    if hash != cache.last_hash || last_modified == 0 || last_modified > cache.last_install_time {
        env::set_var("MOON_INSTALLING_DEPS", install_key);

        debug!(
            target: LOG_TARGET,
            "Installing {} dependencies in {}",
            runtime.label(),
            color::path(working_dir)
        );

        workspace.cache.create_hash_manifest(&hash, &hashset)?;

        platform
            .install_deps(&context, runtime, working_dir)
            .await?;

        cache.last_hash = hash;
        cache.last_install_time = time::now_millis();
        cache.save()?;

        env::remove_var("MOON_INSTALLING_DEPS");

        return Ok(ActionStatus::Passed);
    }

    debug!(
        target: LOG_TARGET,
        "Lockfile has not changed since last install, skipping install",
    );

    Ok(ActionStatus::Skipped)
}
