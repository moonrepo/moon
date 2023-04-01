use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_emitter::Emitter;
use moon_logger::{color, debug};
use moon_platform::Runtime;
use moon_project::Project;
use moon_runner::Runner;
use moon_target::Target;
use moon_terminal::Checkpoint;
use moon_workspace::Workspace;
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
) -> Result<ActionStatus, PipelineError> {
    env::set_var("MOON_RUNNING_ACTION", "run-target");

    let emitter = emitter.read().await;
    let workspace = workspace.read().await;
    let task = project.get_task(&target.task_id)?;
    let mut runner = Runner::new(&emitter, &workspace, project, task)?;

    debug!(
        target: LOG_TARGET,
        "Running target {}",
        color::target(&task.target)
    );

    let is_no_op = task.is_no_op();

    // If the VCS root does not exist (like in a Docker container),
    // we should avoid failing and simply disable caching.
    let is_cache_enabled = task.options.cache && workspace.vcs.is_enabled();

    // We must give this task a fake hash for it to be considered complete
    // for other tasks! This case triggers for noop or cache disabled tasks.
    if is_no_op || !is_cache_enabled {
        dbg!(&target.id, "WRITE 1");
        let mut ctx = context.write().await;
        dbg!(&target.id, "WRITE 2");
        ctx.target_hashes.insert(target.clone(), "skipped".into());
        dbg!(&target.id, "WRITE 3");
    }

    // Abort early if a no operation
    if is_no_op {
        debug!(
            target: LOG_TARGET,
            "Target {} is a no operation, skipping",
            color::target(&task.target),
        );

        runner.print_checkpoint(Checkpoint::RunPassed, &["no op"])?;
        runner.flush_output()?;

        return Ok(ActionStatus::Passed);
    }

    // Abort early if this build has already been cached/hashed
    if is_cache_enabled {
        dbg!(&target, "CACHE");

        let mut ctx = context.write().await;

        if let Some(cache_location) = runner.is_cached(&mut ctx, runtime).await? {
            return Ok(runner.hydrate(cache_location).await?);
        }
    } else {
        debug!(
            target: LOG_TARGET,
            "Cache disabled for target {}",
            color::target(&task.target),
        );
    }

    dbg!("BEFORE");

    // Create the command to run based on the task
    let context = context.read().await;
    let mut command = runner.create_command(&context, runtime).await?;

    // Execute the command and return the number of attempts
    let attempts = runner.run_command(&context, &mut command).await?;
    let status = if action.set_attempts(attempts) {
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
