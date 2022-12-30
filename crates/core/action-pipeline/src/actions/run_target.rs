use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_emitter::Emitter;
use moon_logger::{color, debug, warn};
use moon_platform::Runtime;
use moon_project::Project;
use moon_runner::Runner;
use moon_task::Target;
use moon_terminal::Checkpoint;
use moon_workspace::Workspace;
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
    let emitter = emitter.read().await;
    let workspace = workspace.read().await;
    let task = project.get_task(&target.task_id)?;
    let mut runner = Runner::new(&emitter, &workspace, project, task)?;

    debug!(
        target: LOG_TARGET,
        "Running target {}",
        color::id(&task.target)
    );

    // Abort early if a no operation
    if runner.is_no_op() {
        debug!(
            target: LOG_TARGET,
            "Target {} is a no operation, skipping",
            color::id(&task.target),
        );

        runner.print_checkpoint(Checkpoint::RunPassed, &["no op"])?;
        runner.flush_output()?;

        return Ok(ActionStatus::Passed);
    }

    let mut should_cache = task.options.cache;

    // If the VCS root does not exist (like in a Docker image),
    // we should avoid failing and instead log a warning.
    if !workspace.vcs.is_enabled() {
        should_cache = false;

        warn!(
            target: LOG_TARGET,
            "VCS root not found, caching will be disabled!"
        );
    }

    // Abort early if this build has already been cached/hashed
    if should_cache {
        let mut context = context.write().await;

        if let Some(cache_location) = runner.is_cached(&mut context).await? {
            return Ok(runner.hydrate(cache_location).await?);
        }
    }

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
    if should_cache {
        runner.archive_outputs().await?;
    }

    Ok(status)
}
