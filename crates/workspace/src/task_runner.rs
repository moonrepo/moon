use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::task_result::{TaskResult, TaskResultStatus};
use crate::tasks::{install_node_deps, run_target, setup_toolchain, sync_project};
use crate::workspace::Workspace;
use moon_logger::{debug, trace};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

async fn run_task(workspace: Arc<RwLock<Workspace>>, node: &Node) -> Result<(), WorkspaceError> {
    match node {
        Node::InstallNodeDeps => {
            install_node_deps(workspace).await?;
        }
        Node::RunTarget(target_id) => {
            run_target(workspace, target_id).await?;
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

pub struct TaskRunner {}

impl TaskRunner {
    pub fn default() -> Self {
        debug!(
            target: "moon:task-runner",
            "Creating task runner",
        );

        TaskRunner {}
    }

    pub async fn run(
        &self,
        workspace: Workspace,
        graph: DepGraph,
    ) -> Result<Vec<TaskResult>, WorkspaceError> {
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let workspace = Arc::new(RwLock::new(workspace));
        let graph = Arc::new(RwLock::new(graph));

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
                let workspace_clone = Arc::clone(&workspace);
                let graph_clone = Arc::clone(&graph);

                trace!(
                    target: &format!("moon:task-runner:batch:{}:{}", batch_count, task_count),
                    "Running task",
                );

                // TODO - abort parallel threads when an error occurs in a sibling thread
                task_handles.push(task::spawn(async move {
                    let mut result = TaskResult::new(task);
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(task) {
                        match run_task(workspace_clone, node).await {
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
                    Ok(Err(e)) => return Err(e),
                    Err(e) => return Err(WorkspaceError::TaskRunnerFailure(e)),
                }
            }
        }

        Ok(results)
    }
}
