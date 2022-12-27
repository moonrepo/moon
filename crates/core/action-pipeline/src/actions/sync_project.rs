use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_project::{Project, ProjectError};
use moon_project_graph::ProjectGraph;
use moon_utils::is_ci;
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action-pipeline:sync-project";

pub async fn sync_project(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
    project: &Project,
) -> Result<ActionStatus, ProjectError> {
    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    // Collect all project dependencies so we can pass them along.
    // We can't pass the graph itself because of circuler references between crates!
    let mut dependencies = FxHashMap::default();

    for (dep_id, _) in &project.dependencies {
        dependencies.insert(dep_id.to_owned(), project_graph.get(dep_id)?);
    }

    // Sync the projects and return true if any files have been mutated
    let mutated_files = workspace
        .platforms
        .get(project.language)?
        .sync_project(project, &dependencies)
        .await?;

    // If files have been modified in CI, we should update the status to warning,
    // as these modifications should be committed to the repo!
    if mutated_files && is_ci() {
        return Ok(ActionStatus::Invalid);
    }

    Ok(ActionStatus::Passed)
}
