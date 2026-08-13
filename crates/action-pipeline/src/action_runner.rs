use crate::event_emitter::Event;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::actions::*;
use moon_app_context::AppContext;
use moon_common::color;
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn run_action(
    action: &mut Action,
    action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    job_context: JobContext,
) -> miette::Result<()> {
    let JobContext {
        emitter,
        workspace_graph,
        ..
    } = job_context;

    action.start();

    let node = Arc::clone(&action.node);
    let log_label = color::muted_light(&action.label);

    debug!(index = action.node_index, "Running action {}", log_label);

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

            let result = setup_toolchain(action, action_context, app_context, inner).await;

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
                job_context.daemon_client.clone(),
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
        debug!(
            index = action.node_index,
            status = ?action.status,
            "Failed to run action {}",
            log_label,
        );

        // If these actions failed, we should abort instead of trying to continue
        if should_abort_on_failure(&node) {
            action.abort();
        }
    } else {
        debug!(
            index = action.node_index,
            status = ?action.status,
            "Ran action {} in {:?}",
            log_label,
            action.get_duration()
        );
    }

    Ok(())
}

// Provisioning failures poison everything downstream: dependents are
// dispatched on completion (not success), so they would run in a broken
// environment and fail with errors that mask the root cause
fn should_abort_on_failure(node: &ActionNode) -> bool {
    matches!(
        node,
        ActionNode::SetupProto { .. }
            | ActionNode::SetupToolchain { .. }
            | ActionNode::SetupEnvironment { .. }
            | ActionNode::InstallDependencies { .. }
    )
}

fn extract_error<T>(result: &miette::Result<T>) -> Option<String> {
    match result {
        Ok(_) => None,
        Err(error) => Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moon_action::{
        InstallDependenciesNode, RunTaskNode, SetupEnvironmentNode, SetupToolchainNode,
        SyncProjectNode,
    };
    use moon_common::Id;
    use moon_config::UnresolvedVersionSpec;
    use moon_task::Target;
    use moon_toolchain::{ToolchainSpec, VersionSpec};

    #[test]
    fn aborts_for_provisioning_failures() {
        assert!(should_abort_on_failure(&ActionNode::setup_proto(
            VersionSpec::parse("1.0.0").unwrap()
        )));
        assert!(should_abort_on_failure(&ActionNode::setup_toolchain(
            SetupToolchainNode {
                toolchain: ToolchainSpec::new(
                    Id::raw("tc"),
                    UnresolvedVersionSpec::parse("1.0.0").unwrap()
                ),
            }
        )));
        assert!(should_abort_on_failure(&ActionNode::setup_environment(
            SetupEnvironmentNode {
                project_id: None,
                root: "".into(),
                toolchain_id: Id::raw("tc"),
            }
        )));
        assert!(should_abort_on_failure(&ActionNode::install_dependencies(
            InstallDependenciesNode {
                members: None,
                project_id: None,
                root: "".into(),
                toolchain_id: Id::raw("tc"),
            }
        )));
    }

    #[test]
    fn doesnt_abort_for_other_failures() {
        assert!(!should_abort_on_failure(&ActionNode::sync_workspace()));
        assert!(!should_abort_on_failure(&ActionNode::sync_project(
            SyncProjectNode {
                project_id: Id::raw("project"),
            }
        )));
        assert!(!should_abort_on_failure(&ActionNode::run_task(
            RunTaskNode::new(Target::parse("project:task").unwrap())
        )));
    }
}
