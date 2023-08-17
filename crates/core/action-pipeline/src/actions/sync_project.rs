use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_logger::debug;
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_utils::is_ci;
use moon_workspace::Workspace;
use rustc_hash::FxHashMap;
use starbase_styles::color;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:sync-project";

pub async fn sync_project(
    _action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
    project: &Project,
    runtime: &Runtime,
) -> miette::Result<ActionStatus> {
    env::set_var("MOON_RUNNING_ACTION", "sync-project");

    let workspace = workspace.read().await;
    let context = context.read().await;
    let project_graph = project_graph.read().await;

    debug!(
        target: LOG_TARGET,
        "Syncing project {}",
        color::id(&project.id)
    );

    // Create a snapshot for tasks to reference
    workspace
        .cache_engine
        .write_state(project.get_cache_dir().join("snapshot.json"), project)?;

    // Collect all project dependencies so we can pass them along.
    // We can't pass the graph itself because of circuler references between crates!
    let mut dependencies = FxHashMap::default();

    for dep_id in project.dependencies.keys() {
        dependencies.insert(dep_id.to_owned(), project_graph.get(dep_id)?);
    }

    // Sync the projects and return true if any files have been mutated
    let mutated_files = PlatformManager::read()
        .get(runtime)?
        .sync_project(&context, project, &dependencies)
        .await?;

    // If files have been modified in CI, we should update the status to warning,
    // as these modifications should be committed to the repo!
    if mutated_files && is_ci() {
        return Ok(ActionStatus::Invalid);
    }

    Ok(ActionStatus::Passed)
}
