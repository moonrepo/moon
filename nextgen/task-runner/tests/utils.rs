#![allow(dead_code)]

use moon_action::{ActionNode, RunTaskNode};
use moon_platform::{PlatformManager, Runtime};
use moon_project::Project;
use moon_task::Task;
use moon_task_runner::output_archiver::OutputArchiver;
use moon_task_runner::output_hydrater::OutputHydrater;
use moon_test_utils2::{
    generate_platform_manager_from_sandbox, generate_project_graph_from_sandbox, ProjectGraph,
};
use moon_workspace::Workspace;
use proto_core::ProtoEnvironment;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::path::Path;
use std::sync::Arc;

pub fn create_workspace(root: &Path) -> Workspace {
    Workspace::load_from(root, ProtoEnvironment::new_testing(root)).unwrap()
}

pub fn create_node(task: &Task) -> ActionNode {
    ActionNode::RunTask(Box::new(RunTaskNode::new(
        task.target.clone(),
        Runtime::system(),
    )))
}

pub struct TaskRunnerContainer {
    pub sandbox: Sandbox,
    pub platform_manager: PlatformManager,
    pub project_graph: ProjectGraph,
    pub project: Arc<Project>,
    pub workspace: Workspace,
}

impl TaskRunnerContainer {
    pub async fn new_for_project(fixture: &str, project_id: &str) -> Self {
        let sandbox = create_sandbox(fixture);
        let workspace = create_workspace(sandbox.path());
        let project_graph = generate_project_graph_from_sandbox(sandbox.path()).await;
        let project = project_graph.get(project_id).unwrap();
        let platform_manager = generate_platform_manager_from_sandbox(sandbox.path()).await;

        Self {
            sandbox,
            platform_manager,
            project_graph,
            project,
            workspace,
        }
    }

    pub async fn new_for_os(fixture: &str) -> Self {
        Self::new_for_project(fixture, if cfg!(windows) { "windows" } else { "unix" }).await
    }

    pub async fn new(fixture: &str) -> Self {
        Self::new_for_project(fixture, "project").await
    }

    pub fn create_archiver(&self, task_id: &str) -> OutputArchiver {
        let task = self.project.get_task(task_id).unwrap();

        OutputArchiver {
            project_config: &self.project.config,
            task,
            workspace: &self.workspace,
        }
    }

    pub fn create_hydrator(&self, task_id: &str) -> OutputHydrater {
        let task = self.project.get_task(task_id).unwrap();

        OutputHydrater {
            task,
            workspace: &self.workspace,
        }
    }
}
