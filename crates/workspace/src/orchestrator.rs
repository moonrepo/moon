use crate::errors::WorkspaceError;
use crate::jobs::install_node_deps::install_node_deps;
use crate::jobs::run_target::run_target;
use crate::jobs::setup_toolchain::setup_toolchain;
use crate::jobs::sync_project::sync_project;
use crate::work_graph::{JobType, WorkGraph};
use crate::workspace::Workspace;
use rayon;

pub struct Orchestrator<'a> {
    workspace: &'a mut Workspace,
}

impl<'a> Orchestrator<'a> {
    pub fn new(workspace: &'a mut Workspace) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();

        Orchestrator { workspace }
    }

    // pub async fn run(work_graph: &WorkGraph) -> Result<(), WorkspaceError> {}

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
