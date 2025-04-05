use moon_action::{Action, ActionStatus, RunTaskNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_task_runner::TaskRunner;
use moon_workspace_graph::WorkspaceGraph;
use std::sync::Arc;
use tracing::{instrument, warn};

#[instrument(skip(action, action_context, app_context, workspace_graph))]
pub async fn run_task(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    node: &RunTaskNode,
) -> miette::Result<ActionStatus> {
    let project_id = node
        .target
        .get_project_id()
        .expect("Project required for running tasks!");
    let project = workspace_graph.get_project(project_id)?;
    let task = workspace_graph.get_task(&node.target)?;

    // Must be set before running the task in case it fails and
    // and error is bubbled up the stack
    action.allow_failure = task.options.allow_failure;

    let result = TaskRunner::new(&app_context, &project, &task)?
        .run(&action_context, &action.node)
        .await?;

    action.flaky = result.operations.is_flaky();
    action.status = result.operations.get_final_status();
    action.operations = result.operations;

    if action.has_failed() && action.allow_failure {
        warn!(
            "Task {} has failed, but is marked to allow failures, continuing pipeline",
            color::label(&task.target),
        );
    }

    match result.error {
        Some(error) => Err(error),
        None => Ok(action.status),
    }
}
