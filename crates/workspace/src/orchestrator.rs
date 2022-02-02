use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::workspace::Workspace;
use rayon::{ThreadPool, ThreadPoolBuilder};

async fn run_job(workspace: &mut Workspace, node: &Node) -> Result<(), WorkspaceError> {
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

pub struct Orchestrator {
    pool: ThreadPool,
}

impl Orchestrator {
    pub fn new() -> Self {
        Orchestrator {
            pool: ThreadPoolBuilder::new().build().unwrap(),
        }
    }

    pub fn run(&self, workspace: &mut Workspace, graph: &DepGraph) -> Result<(), WorkspaceError> {
        for batch in graph.sort_batched_topological()? {
            // Process a batch of jobs within a Rayon scope. This scope will complete
            // once all jobs complete as their own thread in the pool.
            self.pool.scope_fifo(move |scope| {
                for job in batch {
                    scope.spawn_fifo(move |s| {
                        if let Some(node) = graph.get_node_from_index(job) {
                            run_job(workspace, node);
                        }
                    });
                }
            });
        }

        Ok(())
    }
}
