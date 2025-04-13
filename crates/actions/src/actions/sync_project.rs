use crate::operations::convert_plugin_sync_operation_with_output;
use crate::utils::should_skip_action_matching;
use moon_action::{Action, ActionStatus, SyncProjectNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_ci};
use moon_pdk_api::SyncProjectInput;
use moon_platform::PlatformManager;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tracing::{debug, instrument, warn};

#[instrument(skip(action, action_context, app_context, workspace_graph))]
pub async fn sync_project(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
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
    let mut dependency_fragments = vec![];

    for dep_config in &project.dependencies {
        let dep_project = workspace_graph.get_project(&dep_config.id)?;

        dependency_fragments.push({
            let mut fragment = dep_project.to_fragment();
            fragment.dependency_scope = Some(dep_config.scope);
            fragment
        });

        dependencies.insert(dep_config.id.to_owned(), dep_project);
    }

    // Sync the projects and return true if any files have been mutated
    let mut mutated_files = false;

    // Loop through legacy platforms
    for toolchain_id in project.get_enabled_toolchains() {
        if let Ok(platform) = PlatformManager::read().get_by_toolchain(toolchain_id) {
            if platform
                .sync_project(&action_context, &project, &dependencies)
                .await?
            {
                mutated_files = true;
            }
        }
    }

    // Loop through each toolchain and sync
    for sync_result in app_context
        .toolchain_registry
        .sync_project_many(project.get_enabled_toolchains(), |registry, toolchain| {
            SyncProjectInput {
                context: registry.create_context(),
                project_dependencies: dependency_fragments.clone(),
                project: project.to_fragment(),
                toolchain_config: registry.create_merged_config(
                    &toolchain.id,
                    &app_context.toolchain_config,
                    &project.config,
                ),
            }
        })
        .await?
    {
        if !sync_result.output.changed_files.is_empty() {
            mutated_files = true;
        }

        action
            .operations
            .push(convert_plugin_sync_operation_with_output(
                sync_result.operation,
                sync_result.output,
            ));
    }

    // TODO track changed files and print them

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
