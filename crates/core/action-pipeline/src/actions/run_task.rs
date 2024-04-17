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
use std::time::SystemTime;
use tokio::sync::{Mutex, RwLock};

const LOG_TARGET: &str = "moon:action:run-task";

#[allow(clippy::too_many_arguments)]
pub async fn run_task(
    action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    emitter: Arc<RwLock<Emitter>>,
    workspace: Arc<RwLock<Workspace>>,
    console: Arc<Console>,
    project: &Project,
    target: &Target,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "run-task");

    let emitter = emitter.read().await;
    let workspace = workspace.read().await;
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
        let mut ctx = context.write().await;

        for dep in &task.deps {
            if let Some(dep_state) = ctx.target_states.get(&dep.target) {
                if !dep_state.is_complete() {
                    ctx.target_states
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
        let mut ctx = context.write().await;

        if let Some(cache_location) = runner.is_cached(&mut ctx, runtime).await? {
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
            .write()
            .await
            .target_states
            .insert(target.clone(), TargetState::Passthrough);
    }

    let attempts_result = {
        let _ctx: RwLock<ActionContext>;
        let ctx = if is_cache_enabled {
            context.read().await
        } else {
            // Concurrent long-running tasks will cause a deadlock, as some threads will
            // attempt to write to context while others are reading from it, and long-running
            // tasks may never release the lock. Unfortuantely we have to clone here to work
            // around it, so revisit in the future.
            _ctx = RwLock::new(context.read().await.clone());
            _ctx.read().await
        };

        if let Some(mutex_name) = &task.options.mutex {
            if !ctx.named_mutexes.contains_key(mutex_name) {
                ctx.named_mutexes
                    .insert(mutex_name.to_owned(), Arc::new(Mutex::new(())));
            }

            debug!(
                target: LOG_TARGET,
                "Waiting to acquire {} mutex lock for {} before running ({:?})",
                color::id(mutex_name),
                color::label(&task.target),
                SystemTime::now()
            );

            if let Some(named_mutex) = ctx.named_mutexes.get(mutex_name) {
                let _guard = named_mutex.lock().await;

                debug!(
                    target: LOG_TARGET,
                    "Acquired {} mutex lock for {} ({:?})",
                    color::id(mutex_name),
                    color::label(&task.target),
                    SystemTime::now()
                );

                runner.create_and_run_command(&ctx, runtime).await
            } else {
                Result::Err(miette::Report::msg(format!(
                    "Unable to acquire named mutex \"{}\"",
                    mutex_name
                )))
            }
        } else {
            runner.create_and_run_command(&ctx, runtime).await
        }
    };

    match attempts_result {
        Ok(attempts) => {
            let status = if action.set_attempts(attempts, &task.command) {
                ActionStatus::Passed
            } else {
                context
                    .write()
                    .await
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
                .write()
                .await
                .target_states
                .insert(target.clone(), TargetState::Failed);

            Err(err)
        }
    }
}
