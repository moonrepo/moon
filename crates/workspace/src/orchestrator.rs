use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::workspace::Workspace;
use awaitgroup::WaitGroup;
use moon_logger::{color, debug, error, trace};
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

pub struct Orchestrator {}

impl Orchestrator {
    pub fn default() -> Self {
        debug!(
            target: "moon:orchestrator",
            "Creating orchestrator",
        );

        Orchestrator {}
    }

    pub async fn run<'a>(
        &self,
        workspace: Workspace,
        graph: DepGraph,
    ) -> Result<(), WorkspaceError> {
        let batches = graph.sort_batched_topological()?;
        let batches_count = batches.len();
        let workspace = Arc::new(RwLock::new(workspace));
        let graph = Arc::new(RwLock::new(graph));

        println!("{:#?}", batches);

        for (b, batch) in batches.into_iter().enumerate() {
            let batch_count = b + 1;
            let jobs_count = batch.len();

            trace!(
                target: "moon:orchestrator:batch",
                "[{}/{}] ▸▸▸", batch_count, batches_count
            );

            let mut wait_group = WaitGroup::new();

            for (j, job) in batch.into_iter().enumerate() {
                let job_count = j + 1;
                let worker = wait_group.worker();
                let workspace_clone = Arc::clone(&workspace);
                let graph_clone = Arc::clone(&graph);

                task::spawn(async move {
                    trace!(
                        target: "moon:orchestrator:batch:job",
                        "[{}/{}] ▸▸▸", job_count, jobs_count
                    );

                    let own_graph = graph_clone.read().await;

                    if let Some(node) = own_graph.get_node_from_index(job) {
                        match run_job(workspace_clone, node).await {
                            Ok(_) => {}
                            Err(e) => {
                                error!(
                                    target: "moon:orchestrator:batch:job",
                                    "Failed to run job {:?}: {}", job, e
                                );
                            }
                        }
                    } else {
                        trace!(
                            target: "moon:orchestrator:batch:job",
                            "Node not found with index {:?}", job
                        );
                    }

                    worker.done();

                    trace!(
                        target: "moon:orchestrator:batch:job",
                        "[{}/{}] ◂◂◂", job_count, jobs_count
                    );
                });
            }

            // Wait for all jobs in this batch to complete
            wait_group.wait().await;

            trace!(
                target: "moon:orchestrator:batch",
                "[{}/{}] ◂◂◂", batch_count, batches_count
            );
        }

        Ok(())
    }
}
