use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tracing::trace;

pub async fn run_action(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<()> {
    action.start();

    let node = Arc::clone(&action.node);
    let log_label = color::muted_light(&action.label);

    trace!("Running action {}", log_label);

    // TODO emit started event

    app_context.console.reporter.on_action_started(&action)?;

    dbg!(&action.label);

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
                .on_action_completed(&action, None)?;
        }
        Err(error) => {
            action.finish(ActionStatus::Failed);

            app_context
                .console
                .reporter
                .on_action_completed(&action, Some(&error))?;

            action.fail(error);
        }
    };

    // TODO emit finished event

    if action.has_failed() {
        trace!(
            "Failed to run action {} in {:?}",
            log_label,
            action.duration.unwrap()
        );

        // If these actions failed, we should abort instead of trying to continue
        if matches!(
            *node,
            ActionNode::SetupTool { .. } | ActionNode::InstallDeps { .. }
        ) {
            action.abort();
        }
    } else {
        trace!("Ran action {} in {:?}", log_label, action.duration.unwrap());
    }

    Ok(())
}
