use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::task_result::TaskResult;
use crate::tasks::{install_node_deps, run_target, setup_toolchain, sync_project};
use crate::workspace::Workspace;
use awaitgroup::WaitGroup;
use moon_logger::{debug, error, trace};
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

        let results: Vec<TaskResult> = vec![];

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let tasks_count = batch.len();

            trace!(
                target: &format!("moon:task-runner:batch:{}", batch_count),
                "Running {} tasks",
                tasks_count
            );

            let mut wait_group = WaitGroup::new();

            for (t, task) in batch.into_iter().enumerate() {
                let task_count = t + 1;
                let worker = wait_group.worker();
                let workspace_clone = Arc::clone(&workspace);
                let graph_clone = Arc::clone(&graph);

                trace!(
                    target: &format!("moon:task-runner:batch:{}:{}", batch_count, task_count),
                    "Running task",
                );

                task::spawn(async move {
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(task) {
                        match run_task(workspace_clone, node).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!(
                                    target: "moon:task-runner:batch:task",
                                    "Failed to run task {:?}: {}", task, e
                                );
                            }
                        }
                    } else {
                        trace!(
                            target: "moon:task-runner:batch:task",
                            "Node not found with index {:?}", task
                        );
                    }

                    worker.done();
                });
            }

            // Wait for all tasks in this batch to complete
            wait_group.wait().await;
        }

        Ok(results)
    }
}
