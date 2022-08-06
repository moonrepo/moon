use crate::action::{Action, ActionStatus};
use crate::context::ActionContext;
use crate::errors::ActionError;
use crate::target::{node, system, CacheLocation, TargetRunner};
use moon_config::PlatformType;
use moon_logger::{color, debug};
use moon_task::Target;
use moon_terminal::Checkpoint;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:run-target";

pub async fn run_target(
    action: &mut Action,
    context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    target_id: &str,
) -> Result<ActionStatus, ActionError> {
    debug!(
        target: LOG_TARGET,
        "Running target {}",
        color::id(target_id)
    );

    let (project_id, task_id) = Target::parse(target_id)?.ids()?;
    let workspace = workspace.read().await;
    let project = workspace.projects.load(&project_id)?;
    let task = project.get_task(&task_id)?;
    let mut runner = TargetRunner::new(&workspace, &project, task, target_id).await?;

    // Abort early if a no operation
    if runner.is_no_op() {
        debug!(
            target: LOG_TARGET,
            "Target {} is a no operation, skipping",
            color::id(target_id),
        );

        runner.print_checkpoint(Checkpoint::Pass, "(no op)");

        return Ok(ActionStatus::Passed);
    }

    // Abort early if this build has already been cached/hashed
    if task.options.cache {
        let common_hasher = runner.create_common_hasher(context).await?;

        let platform_hasher = match task.platform {
            PlatformType::Node => node::create_target_hasher(&workspace, &project)?,
            _ => node::create_target_hasher(&workspace, &project)?,
        };

        if let Some(cache_location) = runner.is_cached(common_hasher, platform_hasher).await? {
            // Only hydrate when the hash is different from the previous build,
            // as we can assume the outputs from the previous build still exist?
            if matches!(cache_location, CacheLocation::Local) {
                runner.hydrate_outputs().await?;
            }

            runner.print_checkpoint(Checkpoint::Pass, "(cached)");
            runner.print_cache_item();

            return Ok(ActionStatus::Cached);
        }
    }

    // Create the command to run based on the task
    let working_dir = if task.options.run_from_workspace_root {
        &workspace.root
    } else {
        &project.root
    };

    let mut command = match task.platform {
        PlatformType::Node => {
            node::create_target_command(context, &workspace, &project, task).await?
        }
        _ => system::create_target_command(task, working_dir),
    };

    command
        .cwd(working_dir)
        // We need to handle non-zero's manually
        .no_error_on_failure();

    debug!(
        target: LOG_TARGET,
        "Creating {} command (in working directory {})",
        color::target(&task.target),
        color::path(working_dir)
    );

    // Execute the command and return the number of attempts
    action.attempts = Some(runner.run_command(context, &mut command).await?);

    // If successful, cache the task outputs
    if task.options.cache {
        runner.cache_outputs().await?;
    }

    Ok(ActionStatus::Passed)
}
