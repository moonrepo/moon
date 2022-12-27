use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, warn};
use moon_platform::Runtime;
use moon_project::Project;
use moon_utils::{fs, is_offline, time};
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline:install-deps";

pub async fn install_deps(
    _action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    runtime: &Runtime,
    project: Option<&Project>,
) -> Result<ActionStatus, PipelineError> {
    if matches!(runtime, Runtime::System) {
        return Ok(ActionStatus::Skipped);
    }

    let workspace = workspace.read().await;
    let context = context.read().await;

    if is_offline() {
        warn!(
            target: LOG_TARGET,
            "No internet connection, assuming offline and skipping install"
        );

        return Ok(ActionStatus::Skipped);
    }

    // When cache is write only, avoid install as user is typically force updating cache
    if workspace.cache.get_mode().is_write_only() {
        debug!(target: LOG_TARGET, "Force updating cache, skipping install",);

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

    // let Some(depman) = platform.get_dependency_manager(runtime.version())? else  {
    //     debug!(
    //         target: LOG_TARGET,
    //         "No dependency manager for language, skipping install",
    //     );

    //     return Ok(ActionStatus::Skipped);
    // };

    let lockfile = ""; // depman.get_lock_filename();
    let manifest = ""; // depman.get_manifest_filename();

    // Determine the working directory and whether lockfiles and manifests have been modified
    let working_dir;
    let has_modified_files;

    if let Some(project) = project {
        working_dir = &project.root;
        has_modified_files = context.touched_files.contains(&working_dir.join(&lockfile))
            || context.touched_files.contains(&working_dir.join(&manifest));
    } else {
        working_dir = &workspace.root;
        has_modified_files = context
            .touched_files
            .iter()
            .any(|f| f.ends_with(&lockfile) || f.ends_with(&manifest));
    }

    // Install dependencies in the current project or workspace
    let lockfile_path = working_dir.join(&lockfile);
    let mut last_modified = 0;
    let mut cache = workspace
        .cache
        .cache_deps_state(runtime, project.map(|p| p.id.as_ref()))?;

    if lockfile_path.exists() {
        last_modified = time::to_millis(
            fs::metadata(&lockfile_path)?
                .modified()
                .map_err(|e| map_io_to_fs_error(e, lockfile_path.clone()))?,
        );
    }

    // Install deps if the lockfile has been modified since the last time they were installed!
    if has_modified_files || last_modified == 0 || last_modified > cache.last_install_time {
        debug!(
            target: LOG_TARGET,
            "Installing {} dependencies in {}",
            runtime.label(),
            color::path(&working_dir)
        );

        platform
            .install_deps(runtime.version(), &working_dir)
            .await?;

        // Update the cache with the timestamp
        cache.last_install_time = time::now_millis();
        cache.save()?;

        return Ok(ActionStatus::Passed);
    }

    debug!(
        target: LOG_TARGET,
        "Lockfile has not changed since last install, skipping install",
    );

    Ok(ActionStatus::Skipped)
}
