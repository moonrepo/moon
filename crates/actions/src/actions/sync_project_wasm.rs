use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SyncProjectNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_workspace_graph::WorkspaceGraph;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

#[instrument(skip(_action, _action_context, app_context, workspace_graph, node))]
pub async fn sync_project(
    _action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: WorkspaceGraph,
    node: &SyncProjectNode,
) -> miette::Result<ActionStatus> {
    let project_id = &node.project;

    // Skip action if requested too
    if let Some(value) = should_skip_action_matching("MOON_SKIP_SYNC_PROJECT", project_id) {
        debug!(
            env = value,
            "Skipping project {} sync because {} is set",
            color::id(project_id),
            color::symbol("MOON_SKIP_SYNC_PROJECT")
        );

        return Ok(ActionStatus::Skipped);
    }

    // Include tasks for snapshot!
    let project = workspace_graph.get_project_with_tasks(project_id)?;

    // Lock the project to avoid collisions
    let _lock = app_context
        .cache_engine
        .create_lock(format!("syncProject-{}", project.id))?;

    debug!("Syncing project {}", color::id(&project.id));

    // Create a snapshot for tasks to reference
    app_context
        .cache_engine
        .state
        .save_project_snapshot(&project.id, &project)?;

    // Collect all project dependencies so we can pass them along
    let project_dependencies = project
        .dependencies
        .iter()
        .map(|cfg| cfg.id.clone())
        .collect::<Vec<_>>();

    // Loop through each toolchain and sync
    let mut changed_files = vec![];
    let context = app_context.toolchain_registry.create_context();

    for toolchain_id in &project.toolchains {
        if let Ok(toolchain) = app_context.toolchain_registry.load(toolchain_id).await {
            changed_files.extend(
                toolchain
                    .sync_project(
                        project.id.clone(),
                        project_dependencies.clone(),
                        context.clone(),
                    )
                    .await?,
            );
        }
    }

    // If files have been modified in CI, we should update the status to warning,
    // as these modifications should be committed to the repo!
    if !changed_files.is_empty() && is_ci() {
        warn!(
            project_id = project.id.as_str(),
            "Files were changed during project sync that should be committed to the repository"
        );

        return Ok(ActionStatus::Invalid);
    }

    Ok(ActionStatus::Passed)
}
