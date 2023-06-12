use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::sync_codeowners;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> miette::Result<ActionStatus> {
    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    if workspace.config.codeowners.sync_on_run {
        sync_codeowners(&workspace, &project_graph).await?;
    }

    Ok(ActionStatus::Passed)
}
