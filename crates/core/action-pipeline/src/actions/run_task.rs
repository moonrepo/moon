use moon_action::{Action, ActionStatus};
use moon_action_context::{ActionContext, TargetState};
use moon_console::{Checkpoint, Console};
use moon_emitter::Emitter;
use moon_logger::{debug, warn};
use moon_platform::Runtime;
use moon_project::Project;
use moon_runner::Runner;
use moon_target::Target;
use moon_workspace::Workspace;
use starbase_styles::color;
use std::env;
use std::sync::Arc;

const LOG_TARGET: &str = "moon:action:run-task";

#[allow(clippy::too_many_arguments)]
pub async fn run_task(
    action: &mut Action,
    context: Arc<ActionContext>,
    emitter: Arc<Emitter>,
    workspace: Arc<Workspace>,
    console: Arc<Console>,
    project: &Project,
    target: &Target,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "run-task");

    let task = project.get_task(&target.task_id)?;
    let mut runner = Runner::new(&emitter, &workspace, project, task, console)?;

    debug!(
        target: LOG_TARGET,
        "Running task {}",
        color::label(&task.target)
    );

    runner.node = Arc::clone(&action.node);
    action.allow_failure = task.options.allow_failure;

    // If a dependency failed, we should skip this target
    if !task.deps.is_empty() {
        for dep in &task.deps {
            if let Some(dep_state) = context.target_states.get(&dep.target) {
                if !dep_state.is_complete() {
                    context
                        .target_states
                        .insert(target.clone(), TargetState::Skipped);

                    debug!(
                        target: LOG_TARGET,
                        "Dependency {} of {} has failed or has been skipped, skipping this target",
                        color::label(&dep.target),
                        color::label(&task.target)
                    );

                    runner.print_checkpoint(Checkpoint::RunFailed, ["skipped".to_owned()])?;

                    return Ok(ActionStatus::Skipped);
                }
            }
        }
    }

    // If the VCS root does not exist (like in a Docker container),
    // we should avoid failing and simply disable caching.
    let is_cache_enabled = task.options.cache && workspace.vcs.is_enabled();

    // Abort early if this build has already been cached/hashed
    if is_cache_enabled {
        if let Some(cache_location) = runner.is_cached(&context, runtime).await? {
            return runner.hydrate(cache_location).await;
        }
    } else {
        debug!(
            target: LOG_TARGET,
            "Cache disabled for target {}",
            color::label(&task.target),
        );

        // We must give this task a fake hash for it to be considered complete
        // for other tasks! This case triggers for noop or cache disabled tasks.
        context
            .target_states
            .insert(target.clone(), TargetState::Passthrough);
    }

    let attempts_result = {
        if let Some(mutex_name) = &task.options.mutex {
            debug!(
                target: LOG_TARGET,
                "Waiting to acquire {} mutex lock for {} before running",
                color::id(mutex_name),
                color::label(&task.target),
            );

            let mutex = context.get_or_create_mutex(mutex_name);
            let _guard = mutex.lock().await;

            debug!(
                target: LOG_TARGET,
                "Acquired {} mutex lock for {}",
                color::id(mutex_name),
                color::label(&task.target),
            );

            // This is required within this block so that the guard
            // above isn't immediately dropped!
            runner.create_and_run_command(&context, runtime).await
        } else {
            runner.create_and_run_command(&context, runtime).await
        }
    };

    match attempts_result {
        Ok(attempts) => {
            let status = if action.set_attempts(attempts, &task.command) {
                ActionStatus::Passed
            } else {
                context
                    .target_states
                    .insert(target.clone(), TargetState::Failed);

                if action.allow_failure {
                    warn!(
                        target: LOG_TARGET,
                        "Target {} has failed, but is marked to allow failures, continuing pipeline",
                        color::label(&task.target),
                    );
                }

                ActionStatus::Failed
            };

            // If successful, cache the task outputs
            if is_cache_enabled {
                runner.archive_outputs().await?;
            }

            Ok(status)
        }
        Err(err) => {
            context
                .target_states
                .insert(target.clone(), TargetState::Failed);

            Err(err)
        }
    }
}
