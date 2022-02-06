use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::results::Result;
use crate::workspace::Workspace;
use awaitgroup::WaitGroup;
use moon_logger::{debug, error, trace};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task;

async fn run_job(workspace: Arc<RwLock<Workspace>>, node: &Node) -> Result<(), WorkspaceError> {
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
    ) -> Result<Vec<Result>, WorkspaceError> {
        let node_count = graph.graph.node_count();
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let workspace = Arc::new(RwLock::new(workspace));
        let graph = Arc::new(RwLock::new(graph));

        debug!(
            target: "moon:task-runner",
            "Running {} jobs across {} batches", node_count, batches_count
        );

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let jobs_count = batch.len();

            trace!(
                target: &format!("moon:task-runner:batch:{}", batch_count),
                "Running {} jobs",
                jobs_count
            );

            let mut wait_group = WaitGroup::new();

            for (j, job) in batch.into_iter().enumerate() {
                let job_count = j + 1;
                let worker = wait_group.worker();
                let workspace_clone = Arc::clone(&workspace);
                let graph_clone = Arc::clone(&graph);

                trace!(
                    target: &format!("moon:task-runner:batch:{}:{}", batch_count, job_count),
                    "Running job",
                );

                task::spawn(async move {
                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(job) {
                        match run_job(workspace_clone, node).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!(
                                    target: "moon:task-runner:batch:job",
                                    "Failed to run job {:?}: {}", job, e
                                );
                            }
                        }
                    } else {
                        trace!(
                            target: "moon:task-runner:batch:job",
                            "Node not found with index {:?}", job
                        );
                    }

                    worker.done();
                });
            }

            // Wait for all jobs in this batch to complete
            wait_group.wait().await;
        }

        Ok(())
    }
}
