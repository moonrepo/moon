use crate::errors::PipelineError;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

// const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    _workspace: Arc<RwLock<Workspace>>,
    _project_graph: Arc<RwLock<ProjectGraph>>,
) -> Result<ActionStatus, PipelineError> {
    Ok(ActionStatus::Passed)
}
