use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_console::Console;
use moon_emitter::Emitter;
use moon_logger::warn;
use moon_platform::Runtime;
use moon_project::Project;
use moon_target::Target;
use moon_task_runner::TaskRunner;
use moon_workspace::Workspace;
use starbase_styles::color;
use std::env;
use std::sync::Arc;

const LOG_TARGET: &str = "moon:action:run-task";

#[allow(clippy::too_many_arguments)]
pub async fn run_task(
    action: &mut Action,
    context: Arc<ActionContext>,
    _emitter: Arc<Emitter>,
    workspace: Arc<Workspace>,
    console: Arc<Console>,
    project: &Project,
    target: &Target,
    _runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "run-task");

    let task = project.get_task(&target.task_id)?;

    action.allow_failure = task.options.allow_failure;

    let result = TaskRunner::new(&workspace, project, task, console)?
        .run(&context, &action.node)
        .await?;

    action.set_operations(result.operations, &task.command);

    if action.has_failed() && action.allow_failure {
        warn!(
            target: LOG_TARGET,
            "Task {} has failed, but is marked to allow failures, continuing pipeline",
            color::label(&task.target),
        );
    }

    Ok(action.status)
}
