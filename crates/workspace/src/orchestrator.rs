use crate::dep_graph::{DepGraph, Node, NodeIndex};
use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::workspace::{Workspace, Workspace2};
use awaitgroup::WaitGroup;
use futures::future::join_all;
use std::sync::{Arc, RwLock};
use tokio::task;

// async fn run_job(workspace: &Workspace2, node: &Node) -> Result<(), WorkspaceError> {
//     match node {
//         Node::InstallNodeDeps => {
//             install_node_deps(workspace).await?;
//         }
//         Node::RunTarget(target_id) => {
//             run_target(workspace, target_id).await?;
//         }
//         Node::SetupToolchain => {
//             setup_toolchain(workspace).await?;
//         }
//         Node::SyncProject(project_id) => {
//             sync_project(workspace, project_id).await?;
//         }
//     }

//     Ok(())
// }

pub struct Orchestrator {}

impl Orchestrator {
    pub fn new() -> Self {
        Orchestrator {}
    }

    pub async fn run(&self, workspace: Workspace, graph: DepGraph) -> Result<(), WorkspaceError> {
        let batches = graph.sort_batched_topological()?;

        // let workspace = Arc::new(RwLock::new(workspace));
        // let graph = Arc::new(RwLock::new(graph));
        // let ws = &workspace;
        // let gp = &graph;

        for (i, batch) in batches.into_iter().enumerate() {
            println!("running batch {}", i);

            let mut wait_group = WaitGroup::new();

            for job in batch {
                let worker = wait_group.worker();

                task::spawn(async move {
                    println!("\t running job {:?} = {:?}", job, 0);
                    worker.done();
                });
            }

            // Wait for all jobs in this batch to complete
            wait_group.wait().await;

            println!("ran batch {}", i);

            // Process a batch of jobs within a Rayon scope. This scope will complete
            // once all jobs complete as their own thread in the pool.
            // self.pool.scope_fifo(move |scope| {
            //     for job in batch {
            //         scope.spawn_fifo(move |s| {
            //             if let Some(node) = graph.get_node_from_index(job) {
            //                 run_job(workspace, node);
            //             }
            //         });
            //     }
            // });
        }

        Ok(())
    }
}
