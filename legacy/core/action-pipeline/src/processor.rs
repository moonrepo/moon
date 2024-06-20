use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::actions::{
    install_deps, run_task, setup_toolchain, sync_project, sync_workspace,
};
use moon_app_context::AppContext;
use moon_emitter::{Emitter, Event};
use moon_logger::trace;
use moon_project_graph::ProjectGraph;
use starbase_styles::color;
use std::sync::Arc;
use tracing::instrument;

fn extract_error<T>(result: &miette::Result<T>) -> Option<String> {
    match result {
        Ok(_) => None,
        Err(error) => Some(error.to_string()),
    }
}

#[instrument(skip_all)]
pub async fn process_action(
    mut action: Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    emitter: Arc<Emitter>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<Action> {
    action.start();

    let node = Arc::clone(&action.node);
    let app_ctx = Arc::clone(&app_context);
    let log_action_label = color::muted_light(&action.label);

    trace!("Processing action {}", log_action_label);

    emitter
        .emit(Event::ActionStarted {
            action: &action,
            node: &node,
        })
        .await?;

    app_ctx.console.reporter.on_action_started(&action)?;

    let result = match &*node {
        ActionNode::None => Ok(ActionStatus::Skipped),

        // Setup and install the specific tool
        ActionNode::SetupToolchain(inner) => {
            emitter
                .emit(Event::ToolInstalling {
                    runtime: &inner.runtime,
                })
                .await?;

            let setup_result =
                setup_toolchain(&mut action, action_context, app_context, inner).await;

            emitter
                .emit(Event::ToolInstalled {
                    error: extract_error(&setup_result),
                    runtime: &inner.runtime,
                })
                .await?;

            setup_result
        }

        // Install dependencies in the workspace root
        ActionNode::InstallWorkspaceDeps(inner) => {
            emitter
                .emit(Event::DependenciesInstalling {
                    project: None,
                    runtime: &inner.runtime,
                })
                .await?;

            let install_result = install_deps(
                &mut action,
                action_context,
                app_context,
                &inner.runtime,
                None,
            )
            .await;

            emitter
                .emit(Event::DependenciesInstalled {
                    error: extract_error(&install_result),
                    project: None,
                    runtime: &inner.runtime,
                })
                .await?;

            install_result
        }

        // Install dependencies in the project root
        ActionNode::InstallProjectDeps(inner) => {
            let project = project_graph.get(&inner.project)?;

            emitter
                .emit(Event::DependenciesInstalling {
                    project: Some(&project),
                    runtime: &inner.runtime,
                })
                .await?;

            let install_result = install_deps(
                &mut action,
                action_context,
                app_context,
                &inner.runtime,
                Some(&project),
            )
            .await;

            emitter
                .emit(Event::DependenciesInstalled {
                    error: extract_error(&install_result),
                    project: Some(&project),
                    runtime: &inner.runtime,
                })
                .await?;

            install_result
        }

        // Sync a project within the graph
        ActionNode::SyncProject(inner) => {
            let project = project_graph.get(&inner.project)?;

            emitter
                .emit(Event::ProjectSyncing {
                    project: &project,
                    runtime: &inner.runtime,
                })
                .await?;

            let sync_result = sync_project(
                &mut action,
                action_context,
                app_context,
                project_graph,
                inner,
            )
            .await;

            emitter
                .emit(Event::ProjectSynced {
                    error: extract_error(&sync_result),
                    project: &project,
                    runtime: &inner.runtime,
                })
                .await?;

            sync_result
        }

        // Sync the workspace
        ActionNode::SyncWorkspace => {
            emitter.emit(Event::WorkspaceSyncing).await?;

            let sync_result =
                sync_workspace(&mut action, action_context, app_context, project_graph).await;

            emitter
                .emit(Event::WorkspaceSynced {
                    error: extract_error(&sync_result),
                })
                .await?;

            sync_result
        }

        // Run a task within a project
        ActionNode::RunTask(inner) => {
            emitter
                .emit(Event::TargetRunning {
                    action: &action,
                    target: &inner.target,
                })
                .await?;

            let run_result = run_task(
                &mut action,
                action_context,
                app_context,
                project_graph,
                inner,
            )
            .await;

            emitter
                .emit(Event::TargetRan {
                    action: &action,
                    error: extract_error(&run_result),
                    target: &inner.target,
                })
                .await?;

            run_result
        }
    };

    let error_message = extract_error(&result);

    match result {
        Ok(status) => {
            action.finish(status);

            app_ctx
                .console
                .reporter
                .on_action_completed(&action, None)?;
        }
        Err(error) => {
            action.finish(ActionStatus::Failed);

            app_ctx
                .console
                .reporter
                .on_action_completed(&action, Some(&error))?;

            action.fail(error);
        }
    };

    if action.has_failed() {
        // If these fail, we should abort instead of trying to continue
        if matches!(
            *node,
            ActionNode::SetupToolchain { .. } | ActionNode::InstallWorkspaceDeps { .. }
        ) {
            action.abort();
        }
    }

    emitter
        .emit(Event::ActionFinished {
            action: &action,
            error: error_message,
            node: &node,
        })
        .await?;

    if action.has_failed() {
        trace!(
            "Failed to process action {} in {:?}",
            log_action_label,
            action.duration.unwrap()
        );
    } else {
        trace!(
            "Processed action {} in {:?}",
            log_action_label,
            action.duration.unwrap()
        );
    }

    Ok(action)
}
