use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::task_result::{TaskResult, TaskResultStatus};
use crate::tasks::{install_node_deps, run_target, setup_toolchain, sync_project};
use crate::workspace::Workspace;
use moon_logger::{color, debug, trace};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

async fn run_task(
    workspace: Arc<RwLock<Workspace>>,
    node: &Node,
    primary_target: &str,
) -> Result<(), WorkspaceError> {
    match node {
        Node::InstallNodeDeps => {
            install_node_deps(workspace).await?;
        }
        Node::RunTarget(target_id) => {
            run_target(workspace, target_id, primary_target).await?;
        }
        Node::SetupToolchain => {
            setup_toolchain(workspace).await?;
        }
        Node::SyncProject(project_id) => {
            sync_project(workspace, project_id).await?;
        }
    }

    Ok(())
}

pub struct TaskRunner {
    primary_target: String,

    workspace: Arc<RwLock<Workspace>>,
}

impl TaskRunner {
    pub fn new(workspace: Workspace) -> Self {
        debug!(
            target: "moon:task-runner",
            "Creating task runner",
        );

        TaskRunner {
            primary_target: String::new(),
            workspace: Arc::new(RwLock::new(workspace)),
        }
    }

    pub async fn cleanup(&self) -> Result<(), WorkspaceError> {
        let workspace = self.workspace.read().await;

        // Delete all previously created runfiles
        trace!(
            target: "moon:task-runner",
            "Deleting stale runfiles"
        );

        workspace.cache.delete_runfiles().await?;

        Ok(())
    }

    pub async fn run(&self, graph: DepGraph) -> Result<Vec<TaskResult>, WorkspaceError> {
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let graph = Arc::new(RwLock::new(graph));
        let primary_target = Arc::new(self.primary_target.clone());

        // Clean the runner state *before* running tasks instead of after,
        // so that failing or broken builds can dig into and debug the state!
        self.cleanup().await?;

        debug!(
            target: "moon:task-runner",
            "Running {} tasks across {} batches", node_count, batches_count
        );

        let mut results: Vec<TaskResult> = vec![];

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let tasks_count = batch.len();

            trace!(
                target: &format!("moon:task-runner:batch:{}", batch_count),
                "Running {} tasks",
                tasks_count
            );

            let mut task_handles = vec![];

            for (t, task) in batch.into_iter().enumerate() {
                let task_count = t + 1;
                let workspace_clone = Arc::clone(&self.workspace);
                let graph_clone = Arc::clone(&graph);
                let primary_target_clone = Arc::clone(&primary_target);

                // TODO - abort parallel threads when an error occurs in a sibling thread
                task_handles.push(task::spawn(async move {
                    let mut result = TaskResult::new(task);
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(task) {
                        trace!(
                            target:
                                &format!("moon:task-runner:batch:{}:{}", batch_count, task_count),
                            "Running task {}",
                            color::muted_light(&node.label())
                        );

                        match run_task(workspace_clone, node, &primary_target_clone).await {
                            Ok(_) => {
                                result.pass();
                            }
                            Err(error) => {
                                result.fail();

                                return Err(error);
                            }
                        }
                    } else {
                        result.status = TaskResultStatus::Invalid;

                        return Err(WorkspaceError::DepGraphUnknownNode(task.index()));
                    }

                    Ok(result)
                }));
            }

            // Wait for all tasks in this batch to complete,
            // while also handling and propagating errors
            for handle in task_handles {
                match handle.await {
                    Ok(Ok(result)) => results.push(result),
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(e) => {
                        return Err(WorkspaceError::TaskRunnerFailure(e));
                    }
                }
            }
        }

        debug!(
            target: "moon:task-runner",
            "Finished running {} tasks", node_count
        );

        Ok(results)
    }

    pub fn set_primary_target(&mut self, target: &str) -> &mut Self {
        self.primary_target = target.to_owned();
        self
    }
}
