use crate::action::{Action, ActionStatus};
use crate::actions::{install_node_deps, run_target, setup_toolchain, sync_project};
use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::{color, debug, trace};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task;

const TARGET: &str = "moon:task-runner";

async fn run_action(
    workspace: Arc<RwLock<Workspace>>,
    graph_node: &Node,
    primary_target: &str,
    passthrough_args: &[String],
) -> Result<ActionStatus, WorkspaceError> {
    let status;

    match graph_node {
        Node::InstallNodeDeps => {
            status = install_node_deps(workspace).await?;
        }
        Node::RunTarget(target_id) => {
            status = run_target(workspace, target_id, primary_target, passthrough_args).await?;
        }
        Node::SetupToolchain => {
            status = setup_toolchain(workspace).await?;
        }
        Node::SyncProject(project_id) => {
            status = sync_project(workspace, project_id).await?;
        }
    }

    Ok(status)
}

pub struct TaskRunner {
    bail: bool,

    pub duration: Option<Duration>,

    passthrough_args: Vec<String>,

    primary_target: String,

    workspace: Arc<RwLock<Workspace>>,
}

impl TaskRunner {
    pub fn new(workspace: Workspace) -> Self {
        debug!(target: TARGET, "Creating task runner",);

        TaskRunner {
            bail: false,
            duration: None,
            passthrough_args: Vec::new(),
            primary_target: String::new(),
            workspace: Arc::new(RwLock::new(workspace)),
        }
    }

    pub fn bail_on_error(&mut self) -> &mut Self {
        self.bail = true;
        self
    }

    pub async fn cleanup(&self) -> Result<(), WorkspaceError> {
        let workspace = self.workspace.read().await;

        // Delete all previously created runfiles
        trace!(target: TARGET, "Deleting stale runfiles");

        workspace.cache.delete_runfiles().await?;

        Ok(())
    }

    pub async fn run(&mut self, graph: DepGraph) -> Result<Vec<Action>, WorkspaceError> {
        let start = Instant::now();
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let graph = Arc::new(RwLock::new(graph));
        let passthrough_args = Arc::new(self.passthrough_args.clone());
        let primary_target = Arc::new(self.primary_target.clone());

        // Clean the runner state *before* running tasks instead of after,
        // so that failing or broken builds can dig into and debug the state!
        self.cleanup().await?;

        debug!(
            target: TARGET,
            "Running {} tasks across {} batches", node_count, batches_count
        );

        let mut results: Vec<Action> = vec![];

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let tasks_count = batch.len();

            trace!(
                target: &format!("{}:batch:{}", TARGET, batch_count),
                "Running {} tasks",
                tasks_count
            );

            let mut task_handles = vec![];

            for (t, task) in batch.into_iter().enumerate() {
                let task_count = t + 1;
                let workspace_clone = Arc::clone(&self.workspace);
                let graph_clone = Arc::clone(&graph);
                let passthrough_args_clone = Arc::clone(&passthrough_args);
                let primary_target_clone = Arc::clone(&primary_target);

                task_handles.push(task::spawn(async move {
                    let mut result = Action::new(task);
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(task) {
                        result.label = Some(node.label());

                        let log_target_name =
                            format!("{}:batch:{}:{}", TARGET, batch_count, task_count);
                        let log_task_label = color::muted_light(&node.label());

                        trace!(target: &log_target_name, "Running task {}", log_task_label);

                        match run_action(
                            workspace_clone,
                            node,
                            &primary_target_clone,
                            &passthrough_args_clone,
                        )
                        .await
                        {
                            Ok(status) => {
                                result.pass(status);

                                trace!(
                                    target: &log_target_name,
                                    "Ran task {} in {:?}",
                                    log_task_label,
                                    result.duration.unwrap()
                                );
                            }
                            Err(error) => {
                                result.fail(error.to_string());

                                trace!(
                                    target: &log_target_name,
                                    "Task {} failed in {:?}",
                                    log_task_label,
                                    result.duration.unwrap()
                                );
                            }
                        };
                    } else {
                        result.status = ActionStatus::Invalid;

                        return Err(WorkspaceError::DepGraphUnknownNode(task.index()));
                    }

                    Ok(result)
                }));
            }

            // Wait for all tasks in this batch to complete,
            // while also handling and propagating errors
            for handle in task_handles {
                match handle.await {
                    Ok(Ok(result)) => {
                        if self.bail && result.error.is_some() {
                            return Err(WorkspaceError::TaskRunnerFailure(result.error.unwrap()));
                        }

                        results.push(result);
                    }
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(e) => {
                        return Err(WorkspaceError::TaskRunnerFailure(e.to_string()));
                    }
                }
            }
        }

        self.duration = Some(start.elapsed());

        debug!(
            target: TARGET,
            "Finished running {} tasks in {:?}",
            node_count,
            self.duration.unwrap()
        );

        Ok(results)
    }

    pub fn set_passthrough_args(&mut self, args: Vec<String>) -> &mut Self {
        self.passthrough_args = args;
        self
    }

    pub fn set_primary_target(&mut self, target: &str) -> &mut Self {
        self.primary_target = target.to_owned();
        self
    }
}
