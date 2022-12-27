use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_emitter::Emitter;
use moon_logger::{color, debug, warn};
use moon_node_platform::actions as node_actions;
use moon_platform::PlatformType;
use moon_project::Project;
use moon_runner::{HydrateFrom, Runner};
use moon_system_platform::actions as system_actions;
use moon_task::Target;
use moon_terminal::Checkpoint;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline:run-target";

pub async fn run_target(
    action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    emitter: Arc<RwLock<Emitter>>,
    workspace: Arc<RwLock<Workspace>>,
    project: &Project,
    target: &Target,
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
        let common_hasher = runner.create_common_hasher(&context).await?;

        let is_cached = match task.platform {
            PlatformType::Node => {
                runner
                    .is_cached(
                        &mut context,
                        common_hasher,
                        node_actions::create_target_hasher(&workspace, project).await?,
                    )
                    .await?
            }
            PlatformType::System => {
                runner
                    .is_cached(
                        &mut context,
                        common_hasher,
                        system_actions::create_target_hasher(&workspace, project)?,
                    )
                    .await?
            }
            PlatformType::Unknown => None,
        };

        if let Some(cache_location) = is_cached {
            // Only hydrate when the hash is different from the previous build,
            // as we can assume the outputs from the previous build still exist?
            if matches!(cache_location, HydrateFrom::LocalCache)
                || matches!(cache_location, HydrateFrom::RemoteCache)
            {
                runner.hydrate_outputs().await?;
            }

            let mut comments = vec![match cache_location {
                HydrateFrom::LocalCache => "cached",
                HydrateFrom::RemoteCache => "cached from remote",
                HydrateFrom::PreviousOutput => "cached from previous run",
            }];

            if runner.should_print_short_hash() {
                comments.push(runner.get_short_hash());
            }

            runner.print_checkpoint(Checkpoint::RunPassed, &comments)?;
            runner.print_cache_item()?;
            runner.flush_output()?;

            return Ok(if matches!(cache_location, HydrateFrom::RemoteCache) {
                ActionStatus::CachedFromRemote
            } else {
                ActionStatus::Cached
            });
        }
    }

    // Create the command to run based on the task
    let context = context.read().await;
    let mut command = runner.create_command(&context).await?;

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
