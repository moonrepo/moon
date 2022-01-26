use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::work_graph::{JobType, NodeIndex, WorkGraph};
use crate::workspace::Workspace;
use rayon::{ThreadPool, ThreadPoolBuilder};

pub struct Orchestrator<'a> {
    pool: ThreadPool,

    workspace: &'a mut Workspace,
}

impl<'a> Orchestrator<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        let pool = ThreadPoolBuilder::new().num_threads(22).build().unwrap();

        Orchestrator { pool, workspace }
    }

    pub async fn run(work_graph: &'a WorkGraph<'a>) -> Result<(), WorkspaceError> {
        let jobs = work_graph.sort_topological()?;

        Ok(())
    }

    /// Process a batch of jobs within a Rayon scope. This scope will complete
    /// once all jobs complete as their own thread in the pool.
    async fn process_job_batch(&self, work_graph: &'a WorkGraph<'a>, jobs: Vec<NodeIndex>) {
        self.pool.scope(move |s| {
            for job in jobs {
                s.spawn(move |s| {
                    self.run_job(job);
                });
            }
        });
    }

    async fn run_job(&mut self, job: JobType) -> Result<(), WorkspaceError> {
        match job {
            JobType::InstallNodeDeps => {
                install_node_deps(&self.workspace).await?;
            }
            JobType::RunTarget(target_id) => {
                run_target(&self.workspace, target_id).await?;
            }
            JobType::SetupToolchain => {
                setup_toolchain(&self.workspace).await?;
            }
            JobType::SyncProject(project_id) => {
                sync_project(&mut self.workspace, project_id).await?;
            }
        }

        Ok(())
    }
}
