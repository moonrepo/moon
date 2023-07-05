use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_emitter::Emitter;
use moon_logger::debug;
use moon_platform::Runtime;
use moon_project::Project;
use moon_runner::Runner;
use moon_target::Target;
use moon_workspace::Workspace;
use starbase_styles::color;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:run-target";

pub async fn run_target(
    action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    emitter: Arc<RwLock<Emitter>>,
    workspace: Arc<RwLock<Workspace>>,
    project: &Project,
    target: &Target,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "run-target");

    let emitter = emitter.read().await;
    let workspace = workspace.read().await;
    let task = project.get_task(&target.task_id)?;

    debug!(
        target: LOG_TARGET,
        "Running target {}",
        color::label(&task.target)
    );

    // If a dependency failed, we should skip this target
    if !task.deps.is_empty() {
        let mut ctx = context.write().await;

        for dep in &task.deps {
            if let Some(dep_hash) = ctx.target_hashes.get(dep) {
                if dep_hash == "failed" || dep_hash == "skipped" {
                    ctx.target_hashes
                        .insert(task.target.clone(), "skipped".into());

                    debug!(
                        target: LOG_TARGET,
                        "Dependency {} of {} has failed or has been skipped, skipping this target",
                        color::label(&dep),
                        color::label(&task.target)
                    );

                    return Ok(ActionStatus::Skipped);
                }
            }
        }
    }

    let mut runner = Runner::new(&emitter, &workspace, project, task)?;

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
            .target_hashes
            .insert(target.clone(), "passthrough".into());
    }

    let attempts_result = if is_cache_enabled {
        let context = context.read().await;

        runner.create_and_run_command(&context, runtime).await
    } else {
        // Concurrent long-running tasks will cause a deadlock, as some threads will
        // attempt to write to context while others are reading from it, and long-running
        // tasks may never release the lock. Unfortuantely we have to clone here to work
        // around it, so revisit in the future.
        let context = (context.read().await).clone();

        runner.create_and_run_command(&context, runtime).await
    };

    match attempts_result {
        Ok(attempts) => {
            let status = if action.set_attempts(attempts, &task.command) {
                ActionStatus::Passed
            } else {
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
                .target_hashes
                .insert(target.clone(), "failed".into());

            return Err(err);
        }
    }
}
