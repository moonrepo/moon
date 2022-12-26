use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_workspace::{Workspace, WorkspaceError};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline:sync-project";

pub async fn sync_project(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
    project: &Project,
) -> Result<ActionStatus, WorkspaceError> {
    Ok(ActionStatus::Passed)
}
