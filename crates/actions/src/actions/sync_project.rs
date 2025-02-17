use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SyncProjectNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_platform::PlatformManager;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

#[instrument(skip(_action, action_context, app_context, workspace_graph))]
pub async fn sync_project(
    _action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: WorkspaceGraph,
    node: &SyncProjectNode,
) -> miette::Result<ActionStatus> {
    // Include tasks for snapshot!
    let project = workspace_graph.get_project_with_tasks(&node.project)?;

    let _lock = app_context
        .cache_engine
        .create_lock(format!("syncProject-{}", project.id))?;

    if let Some(value) = should_skip_action_matching("MOON_SKIP_SYNC_PROJECT", &project.id) {
        debug!(
            env = value,
            "Skipping project {} sync because {} is set",
            color::id(&project.id),
            color::symbol("MOON_SKIP_SYNC_PROJECT")
        );

        return Ok(ActionStatus::Skipped);
    }

    debug!("Syncing project {}", color::id(&project.id));

    // Create a snapshot for tasks to reference
    app_context
        .cache_engine
        .state
        .save_project_snapshot(&project.id, &project)?;

    // Collect all project dependencies so we can pass them along.
    // We can't pass the graph itself because of circular references between crates!
    let mut dependencies = FxHashMap::default();

    for dep_config in &project.dependencies {
        dependencies.insert(
            dep_config.id.to_owned(),
            workspace_graph.get_project(&dep_config.id)?,
        );
    }

    // Sync the projects and return true if any files have been mutated
    let mutated_files = PlatformManager::read()
        .get_by_toolchain(&node.runtime.toolchain)?
        .sync_project(&action_context, &project, &dependencies)
        .await?;

    // If files have been modified in CI, we should update the status to warning,
    // as these modifications should be committed to the repo!
    if mutated_files && is_ci() {
        warn!(
            project_id = project.id.as_str(),
            "Files were modified during project sync that should be committed to the repository"
        );

        return Ok(ActionStatus::Invalid);
    }

    Ok(ActionStatus::Passed)
}
