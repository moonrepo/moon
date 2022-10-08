use crate::actions;
use crate::emitter::{Event, RunnerEmitter};
use crate::errors::ActionRunnerError;
use moon_action::{Action, ActionContext, ActionStatus};
use moon_contract::Runtime;
use moon_platform_node::actions as node_actions;
use moon_project::ProjectID;
use moon_task::TargetID;
use moon_workspace::Workspace;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Eq, Serialize)]
pub enum ActionNode {
    /// Install tool dependencies in the workspace root.
    InstallDeps(Runtime),

    /// Install tool dependencies in the project root.
    InstallProjectDeps(Runtime, ProjectID),

    /// Run a target (project task).
    RunTarget(TargetID),

    /// Setup a tool + version for the provided platform.
    SetupTool(Runtime),

    /// Sync a project with language specific semantics.
    SyncProject(Runtime, ProjectID),
}

impl ActionNode {
    pub fn label(&self) -> String {
        match self {
            ActionNode::InstallDeps(platform) => match platform {
                Runtime::Node(version) => format!("Install{}Deps({})", platform, version),
                _ => format!("Install{}Deps", platform),
            },
            ActionNode::InstallProjectDeps(platform, id) => match platform {
                Runtime::Node(version) => {
                    format!("Install{}DepsInProject({}, {})", platform, version, id)
                }
                _ => format!("Install{}DepsInProject({})", platform, id),
            },
            ActionNode::RunTarget(id) => format!("RunTarget({})", id),
            ActionNode::SetupTool(platform) => match platform {
                Runtime::Node(version) => format!("Setup{}Tool({})", platform, version),
                _ => format!("Setup{}Tool", platform),
            },
            ActionNode::SyncProject(platform, id) => format!("Sync{}Project({})", platform, id),
        }
    }

    pub async fn run(
        &self,
        action: &mut Action,
        context: &ActionContext,
        workspace: Arc<RwLock<Workspace>>,
        emitter: Arc<RwLock<RunnerEmitter>>,
    ) -> Result<ActionStatus, ActionRunnerError> {
        let local_emitter = Arc::clone(&emitter);
        let local_emitter = local_emitter.read().await;

        match self {
            // Install dependencies in the workspace root
            ActionNode::InstallDeps(runtime) => {
                local_emitter
                    .emit(Event::DependenciesInstalling {
                        project_id: None,
                        runtime,
                    })
                    .await?;

                let install_result = match runtime {
                    Runtime::Node(_) => {
                        node_actions::install_deps(action, context, workspace, runtime, None)
                            .await
                            .map_err(ActionRunnerError::Workspace)
                    }
                    _ => Ok(ActionStatus::Passed),
                };

                local_emitter
                    .emit(Event::DependenciesInstalled {
                        project_id: None,
                        runtime,
                    })
                    .await?;

                install_result
            }

            // Install dependencies in the project root
            ActionNode::InstallProjectDeps(runtime, project_id) => {
                local_emitter
                    .emit(Event::DependenciesInstalling {
                        project_id: Some(project_id),
                        runtime,
                    })
                    .await?;

                let install_result = match runtime {
                    Runtime::Node(_) => node_actions::install_deps(
                        action,
                        context,
                        workspace,
                        runtime,
                        Some(project_id),
                    )
                    .await
                    .map_err(ActionRunnerError::Workspace),
                    _ => Ok(ActionStatus::Passed),
                };

                local_emitter
                    .emit(Event::DependenciesInstalled {
                        project_id: Some(project_id),
                        runtime,
                    })
                    .await?;

                install_result
            }

            // Run a task within a project
            ActionNode::RunTarget(target_id) => {
                local_emitter
                    .emit(Event::TargetRunning { target_id })
                    .await?;

                let run_result = actions::run_target(
                    action,
                    context,
                    workspace,
                    Arc::clone(&emitter),
                    target_id,
                )
                .await;

                local_emitter.emit(Event::TargetRan { target_id }).await?;

                run_result
            }

            // Setup and install the specific tool
            ActionNode::SetupTool(runtime) => {
                local_emitter
                    .emit(Event::ToolInstalling { runtime })
                    .await?;

                let tool_result = actions::setup_toolchain(action, context, workspace, runtime)
                    .await
                    .map_err(ActionRunnerError::Workspace);

                local_emitter.emit(Event::ToolInstalled { runtime }).await?;

                tool_result
            }

            // Sync a project within the graph
            ActionNode::SyncProject(runtime, project_id) => {
                local_emitter
                    .emit(Event::ProjectSyncing {
                        project_id,
                        runtime,
                    })
                    .await?;

                let sync_result = match runtime {
                    Runtime::Node(_) => {
                        node_actions::sync_project(action, context, workspace, project_id)
                            .await
                            .map_err(ActionRunnerError::Workspace)
                    }
                    _ => Ok(ActionStatus::Passed),
                };

                local_emitter
                    .emit(Event::ProjectSynced {
                        project_id,
                        runtime,
                    })
                    .await?;

                sync_result
            }
        }
    }
}

impl PartialEq for ActionNode {
    fn eq(&self, other: &Self) -> bool {
        self.label() == other.label()
    }
}

impl Hash for ActionNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.label().hash(state);
    }
}
