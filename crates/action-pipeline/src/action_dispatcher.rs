use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tracing::trace;

pub async fn dispatch(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<()> {
    action.start();

    let node = Arc::clone(&action.node);
    let log_label = color::muted_light(&action.label);

    trace!("Running action {}", log_label);

    app_context.console.reporter.on_action_started(&action)?;

    let result: miette::Result<ActionStatus> = match &*node {
        ActionNode::None => Ok(ActionStatus::Skipped),
        ActionNode::InstallDeps(_) => todo!(),
        ActionNode::InstallProjectDeps(_) => todo!(),
        ActionNode::RunTask(_) => todo!(),
        ActionNode::SetupTool(_) => todo!(),
        ActionNode::SyncProject(_) => todo!(),
        ActionNode::SyncWorkspace => todo!(),
    };

    Ok(())
}
