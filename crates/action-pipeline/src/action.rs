use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tracing::{instrument, trace};

#[instrument(skip_all)]
pub async fn run_action(
    action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    _project_graph: Arc<ProjectGraph>,
) -> miette::Result<()> {
    action.start();

    let node = Arc::clone(&action.node);
    let log_label = color::muted_light(&action.label);

    trace!(index = action.node_index, "Running action {}", log_label);

    // TODO emit started event

    app_context.console.reporter.on_action_started(action)?;

    let result: miette::Result<ActionStatus> = match &*node {
        ActionNode::None => Ok(ActionStatus::Skipped),
        ActionNode::InstallDeps(_) => Ok(ActionStatus::Passed),
        ActionNode::InstallProjectDeps(_) => Ok(ActionStatus::Passed),
        ActionNode::RunTask(_) => Ok(ActionStatus::Passed),
        ActionNode::SetupTool(_) => Ok(ActionStatus::Passed),
        ActionNode::SyncProject(_) => Ok(ActionStatus::Passed),
        ActionNode::SyncWorkspace => Ok(ActionStatus::Passed),
    };

    match result {
        Ok(status) => {
            action.finish(status);

            app_context
                .console
                .reporter
                .on_action_completed(action, None)?;
        }
        Err(error) => {
            action.finish(ActionStatus::Failed);

            app_context
                .console
                .reporter
                .on_action_completed(action, Some(&error))?;

            action.fail(error);
        }
    };

    if action.has_failed() {
        trace!(
            index = action.node_index,
            status = ?action.status,
            "Failed to run action {}",
            log_label,
        );

        // If these actions failed, we should abort instead of trying to continue
        if matches!(
            *node,
            ActionNode::SetupTool { .. } | ActionNode::InstallDeps { .. }
        ) {
            action.abort();
        }
    } else {
        trace!(
            index = action.node_index,
            status = ?action.status,
            "Ran action {} in {:?}",
            log_label,
            action.get_duration()
        );
    }

    // TODO emit finished event

    Ok(())
}
