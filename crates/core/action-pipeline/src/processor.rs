use crate::actions::install_deps::install_deps;
use crate::actions::setup_tool::setup_tool;
use crate::actions::sync_project::sync_project;
use crate::errors::PipelineError;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_logger::{color, debug, error, trace};
use moon_project_graph::ProjectGraph;
use moon_workspace::Workspace;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn process_action(
    action: &mut Action,
    context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> Result<(), PipelineError> {
    trace!(
        target: &action.log_target,
        "Running action {}",
        color::muted_light(&action.label)
    );

    let local_project_graph = Arc::clone(&project_graph);
    let local_project_graph = local_project_graph.read().await;

    let node = action.node.take().unwrap();
    let result = match &node {
        // Setup and install the specific tool
        ActionNode::SetupTool(runtime) => setup_tool(action, context, workspace, runtime).await,

        // Install dependencies in the workspace root
        ActionNode::InstallDeps(runtime) => {
            install_deps(action, context, workspace, runtime, None).await
        }

        // Install dependencies in the project root
        ActionNode::InstallProjectDeps(runtime, project_id) => {
            let project = local_project_graph.get(project_id)?;

            install_deps(action, context, workspace, runtime, Some(project)).await
        }

        // Sync a project within the graph
        ActionNode::SyncProject(runtime, project_id) => {
            let project = local_project_graph.get(project_id)?;

            sync_project(action, context, workspace, project_graph, project, runtime).await
        }

        // Run a task within a project
        ActionNode::RunTarget(target_id) => Ok(ActionStatus::Skipped),
    };

    match result {
        Ok(status) => {
            action.done(status);
        }
        Err(error) => {
            action.fail(error.to_string());

            // If these fail, we should abort instead of trying to continue
            if matches!(node, ActionNode::SetupTool(_))
                || matches!(node, ActionNode::InstallDeps(_))
            {
                action.abort();
            }
        }
    }

    Ok(())
}
