use crate::utils::should_skip_action_matching;
use miette::IntoDiagnostic;
use moon_action::{Action, ActionStatus, SyncProjectNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_pdk_api::SyncProjectInput;
use moon_platform::PlatformManager;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument, warn};

#[instrument(skip(_action, action_context, app_context, workspace_graph))]
pub async fn sync_project(
    _action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: WorkspaceGraph,
    node: &SyncProjectNode,
) -> miette::Result<ActionStatus> {
    let project_id = &node.project_id;

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
        .create_lock(format!("syncProject-{}", project_id))?;

    debug!("Syncing project {}", color::id(project_id));

    // Create a snapshot for tasks to reference
    app_context
        .cache_engine
        .state
        .save_project_snapshot(project_id, &project)?;

    // Collect all project dependencies so we can pass them along
    let mut dependencies = FxHashMap::default();

    for dep_config in &project.dependencies {
        dependencies.insert(
            dep_config.id.to_owned(),
            workspace_graph.get_project(&dep_config.id)?,
        );
    }

    // Sync the projects and return true if any files have been mutated
    let mut mutated_files = PlatformManager::read()
        .get_by_toolchain(&node.runtime.toolchain)?
        .sync_project(&action_context, &project, &dependencies)
        .await?;

    // Loop through each toolchain and sync
    let mut changed_files = vec![];
    let toolchain_registry = &app_context.toolchain_registry;

    if toolchain_registry.has_plugins() {
        let context = toolchain_registry.create_context();
        let mut set = JoinSet::new();

        for toolchain_id in &project.toolchains {
            if let Ok(toolchain) = toolchain_registry.load(toolchain_id).await {
                if !toolchain.has_func("sync_project").await {
                    continue;
                }

                let input = SyncProjectInput {
                    config: toolchain_registry.create_merged_config(
                        toolchain_id,
                        &app_context.toolchain_config,
                        &project.config,
                    ),
                    context: context.clone(),
                    project_dependencies: dependencies.keys().cloned().collect(),
                    project_id: project.id.clone(),
                };

                set.spawn(async move { toolchain.sync_project(input).await });
            }
        }

        while let Some(result) = set.join_next().await {
            changed_files.extend(result.into_diagnostic()??);
        }

        if !changed_files.is_empty() {
            mutated_files = true;
        }
    }

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
