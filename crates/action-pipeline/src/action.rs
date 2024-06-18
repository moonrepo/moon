use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tracing::trace;

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
        // ActionNode::InstallDeps(_) => todo!(),
        // ActionNode::InstallProjectDeps(_) => todo!(),
        // ActionNode::RunTask(_) => todo!(),
        // ActionNode::SetupTool(_) => todo!(),
        // ActionNode::SyncProject(_) => todo!(),
        // ActionNode::SyncWorkspace => todo!(),
        _ => Ok(ActionStatus::Passed),
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

    // TODO emit finished event

    if action.has_failed() {
        trace!(
            index = action.node_index,
            "Failed to run action {} in {:?}",
            log_label,
            action.get_duration()
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
            "Ran action {} in {:?}",
            log_label,
            action.get_duration()
        );
    }

    Ok(())
}
