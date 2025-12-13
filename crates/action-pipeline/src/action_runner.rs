use crate::event_emitter::{Event, EventEmitter};
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::actions::*;
use moon_app_context::AppContext;
use moon_common::color;
use moon_workspace_graph::WorkspaceGraph;
use std::sync::Arc;
use tracing::{instrument, trace};

#[instrument(skip_all)]
pub async fn run_action(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    workspace_graph: Arc<WorkspaceGraph>,
    emitter: Arc<EventEmitter>,
) -> miette::Result<()> {
    action.start();

    let node = Arc::clone(&action.node);
    let log_label = color::muted_light(&action.label);

    trace!(index = action.node_index, "Running action {}", log_label);

    emitter
        .emit(Event::ActionStarted {
            action,
            node: &node,
        })
        .await?;

    let result = match &*node {
        ActionNode::None => Ok(ActionStatus::Skipped),

        ActionNode::SyncWorkspace => {
            emitter.emit(Event::WorkspaceSyncing).await?;

            let result =
                sync_workspace(action, action_context, app_context, workspace_graph.clone()).await;

            emitter
                .emit(Event::WorkspaceSynced {
                    error: extract_error(&result),
                })
                .await?;

            result
        }

        ActionNode::SyncProject(inner) => {
            let project = workspace_graph.get_project(&inner.project_id)?;

            emitter
                .emit(Event::ProjectSyncing { project: &project })
                .await?;

            let result = sync_project(
                action,
                action_context,
                app_context,
                workspace_graph.clone(),
                inner,
            )
            .await;

            emitter
                .emit(Event::ProjectSynced {
                    error: extract_error(&result),
                    project: &project,
                })
                .await?;

            result
        }

        ActionNode::SetupEnvironment(inner) => {
            let project = match &inner.project_id {
                Some(id) => Some(workspace_graph.get_project(id)?),
                None => None,
            };

            emitter
                .emit(Event::EnvironmentInitializing {
                    project: project.as_deref(),
                    root: &inner.root,
                    toolchain: &inner.toolchain_id,
                })
                .await?;

            let result = setup_environment(
                action,
                action_context,
                app_context,
                workspace_graph.clone(),
                inner,
            )
            .await;

            emitter
                .emit(Event::EnvironmentInitialized {
                    error: extract_error(&result),
                    project: project.as_deref(),
                    root: &inner.root,
                    toolchain: &inner.toolchain_id,
                })
                .await?;

            result
        }

        ActionNode::SetupProto(_) => setup_proto(action, action_context, app_context).await,

        ActionNode::SetupToolchain(inner) => {
            emitter
                .emit(Event::ToolchainInstalling {
                    spec: &inner.toolchain,
                })
                .await?;

            let result = setup_toolchain_plugin(action, action_context, app_context, inner).await;

            emitter
                .emit(Event::ToolchainInstalled {
                    error: extract_error(&result),
                    spec: &inner.toolchain,
                })
                .await?;

            result
        }

        ActionNode::InstallDependencies(inner) => {
            let project = match &inner.project_id {
                Some(id) => Some(workspace_graph.get_project(id)?),
                None => None,
            };

            emitter
                .emit(Event::DependenciesInstalling {
                    project: project.as_deref(),
                    root: Some(&inner.root),
                    toolchain: Some(&inner.toolchain_id),
                })
                .await?;

            let result = install_dependencies(
                action,
                action_context,
                app_context,
                workspace_graph.clone(),
                inner,
            )
            .await;

            emitter
                .emit(Event::DependenciesInstalled {
                    error: extract_error(&result),
                    project: project.as_deref(),
                    root: Some(&inner.root),
                    toolchain: Some(&inner.toolchain_id),
                })
                .await?;

            result
        }

        ActionNode::RunTask(inner) => {
            emitter
                .emit(Event::TaskRunning {
                    node: inner,
                    target: &inner.target,
                })
                .await?;

            let result = run_task(
                action,
                action_context,
                app_context,
                workspace_graph.clone(),
                inner,
            )
            .await;

            emitter
                .emit(Event::TaskRan {
                    error: extract_error(&result),
                    node: inner,
                    target: &inner.target,
                })
                .await?;

            result
        }
    };

    match result {
        Ok(status) => {
            action.finish(status);

            emitter
                .emit(Event::ActionCompleted {
                    action,
                    error: None,
                    error_report: None,
                    node: &node,
                })
                .await?;
        }
        Err(error) => {
            action.finish(ActionStatus::Failed);
            action.fail(error);

            emitter
                .emit(Event::ActionCompleted {
                    action,
                    error: action.error.clone(),
                    error_report: action.error_report.as_ref(),
                    node: &node,
                })
                .await?;
        }
    };

    if action.has_failed() {
        trace!(
            index = action.node_index,
            status = ?action.status,
            "Failed to run action {}",
            log_label,
        );

        // If these actions failed, we should abort instead of trying to continue
        if matches!(
            *node,
            ActionNode::SetupToolchain { .. } | ActionNode::InstallDependencies { .. }
        ) {
            action.abort();
        }
    } else {
        trace!(
            index = action.node_index,
            status = ?action.status,
            "Ran action {} in {:?}",
            log_label,
            action.get_duration()
        );
    }

    Ok(())
}

fn extract_error<T>(result: &miette::Result<T>) -> Option<String> {
    match result {
        Ok(_) => None,
        Err(error) => Some(error.to_string()),
    }
}
