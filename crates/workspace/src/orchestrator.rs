use crate::dep_graph::{DepGraph, Node};
use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::workspace::Workspace;
use rayon::{ThreadPool, ThreadPoolBuilder};

pub struct Orchestrator<'a> {
    pool: ThreadPool,

    workspace: &'a mut Workspace,
}

impl<'a> Orchestrator<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        Orchestrator {
            pool: ThreadPoolBuilder::new().build().unwrap(),
            workspace,
        }
    }

    pub fn run(&self, graph: &'a DepGraph) -> Result<(), WorkspaceError> {
        for batch in graph.sort_batched_topological()? {
            // Process a batch of jobs within a Rayon scope. This scope will complete
            // once all jobs complete as their own thread in the pool.
            self.pool.scope(move |s| {
                for job in batch {
                    s.spawn(move |s| {
                        if let Some(node) = graph.get_node_from_index(job) {
                            self.run_job(node);
                        }
                    });
                }
            });
        }

        Ok(())
    }

    async fn run_job(&mut self, node: &Node) -> Result<(), WorkspaceError> {
        match node {
            Node::InstallNodeDeps => {
                install_node_deps(&self.workspace).await?;
            }
            Node::RunTarget(target_id) => {
                run_target(&self.workspace, target_id).await?;
            }
            Node::SetupToolchain => {
                setup_toolchain(&self.workspace).await?;
            }
            Node::SyncProject(project_id) => {
                sync_project(&mut self.workspace, project_id).await?;
            }
        }

        Ok(())
    }
}
