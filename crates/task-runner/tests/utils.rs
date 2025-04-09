#![allow(dead_code)]

use moon_action::{ActionNode, RunTaskNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_platform::{PlatformManager, Runtime};
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_task_runner::TaskRunner;
use moon_task_runner::command_builder::CommandBuilder;
use moon_task_runner::command_executor::CommandExecutor;
use moon_task_runner::output_archiver::OutputArchiver;
use moon_task_runner::output_hydrater::OutputHydrater;
use moon_test_utils2::{WorkspaceGraph, WorkspaceMocker};
use starbase_archive::Archiver;
use starbase_sandbox::{Sandbox, create_sandbox};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn create_node(task: &Task) -> ActionNode {
    ActionNode::RunTask(Box::new(RunTaskNode::new(
        task.target.clone(),
        Runtime::system(),
    )))
}

pub struct TaskRunnerContainer {
    pub sandbox: Sandbox,
    pub app_context: AppContext,
    pub platform_manager: PlatformManager,
    pub project: Arc<Project>,
    pub project_id: String,
    pub task: Arc<Task>,
    pub task_id: String,
    pub workspace_graph: WorkspaceGraph,
}

impl TaskRunnerContainer {
    pub async fn new_for_project(fixture: &str, project_id: &str, task_id: &str) -> Self {
        let sandbox = create_sandbox(fixture);
        let mocker = WorkspaceMocker::new(sandbox.path())
            .load_default_configs()
            .with_global_envs();

        let app_context = mocker.mock_app_context();
        let workspace_graph = mocker.mock_workspace_graph().await;
        let platform_manager = mocker.mock_platform_manager().await;
        let project = workspace_graph.get_project(project_id).unwrap();
        let task = workspace_graph
            .get_task_from_project(project_id, task_id)
            .unwrap();

        Self {
            sandbox,
            app_context,
            platform_manager,
            workspace_graph,
            project,
            project_id: project_id.to_owned(),
            task,
            task_id: task_id.to_owned(),
        }
    }

    pub async fn new_os(fixture: &str, task_id: &str) -> Self {
        Self::new_for_project(
            fixture,
            if cfg!(windows) { "windows" } else { "unix" },
            task_id,
        )
        .await
    }

    pub async fn new(fixture: &str, task_id: &str) -> Self {
        Self::new_for_project(fixture, "project", task_id).await
    }

    pub fn create_archiver(&self) -> OutputArchiver {
        OutputArchiver {
            app: &self.app_context,
            project: &self.project,
            task: &self.task,
        }
    }

    pub fn create_hydrator(&self) -> OutputHydrater {
        OutputHydrater {
            app: &self.app_context,
            task: &self.task,
        }
    }

    pub async fn create_command(&self, context: ActionContext) -> Command {
        self.create_command_with_config(context, |_, _| {}).await
    }

    pub async fn create_command_with_config(
        &self,
        context: ActionContext,
        mut op: impl FnMut(&mut Task, &mut ActionNode),
    ) -> Command {
        let mut task = self.task.as_ref().to_owned();
        let mut node = create_node(&task);

        op(&mut task, &mut node);

        self.internal_create_command(&context, &task, &node).await
    }

    pub async fn create_command_executor(&self, context: &ActionContext) -> CommandExecutor {
        let node = create_node(&self.task);

        CommandExecutor::new(
            &self.app_context,
            &self.project,
            &self.task,
            &node,
            self.internal_create_command(context, &self.task, &node)
                .await,
        )
    }

    pub fn create_runner(&self) -> TaskRunner {
        let mut runner = TaskRunner::new(&self.app_context, &self.project, &self.task).unwrap();
        runner.set_platform_manager(&self.platform_manager);
        runner
    }

    pub fn create_action_node(&self) -> ActionNode {
        create_node(&self.task)
    }

    pub fn pack_archive(&self) -> PathBuf {
        let sandbox = &self.sandbox;
        let file = sandbox.path().join(".moon/cache/outputs/hash123.tar.gz");

        let out = format!(
            ".moon/cache/states/{}/{}/stdout.log",
            self.project_id, self.task_id,
        );

        let err = format!(
            ".moon/cache/states/{}/{}/stderr.log",
            self.project_id, self.task_id,
        );

        let txt = format!("{}/file.txt", self.project_id);

        sandbox.create_file(&out, "stdout");
        sandbox.create_file(&err, "stderr");
        sandbox.create_file(&txt, "content");

        let mut archiver = Archiver::new(sandbox.path(), &file);
        archiver.add_source_file(&out, None);
        archiver.add_source_file(&err, None);
        archiver.add_source_file(&txt, None);
        archiver.pack_from_ext().unwrap();

        // Remove sources so we can test unpacking
        fs::remove_file(sandbox.path().join(out)).unwrap();
        fs::remove_file(sandbox.path().join(err)).unwrap();
        fs::remove_file(sandbox.path().join(txt)).unwrap();

        file
    }

    async fn internal_create_command(
        &self,
        context: &ActionContext,
        task: &Task,
        node: &ActionNode,
    ) -> Command {
        let mut builder = CommandBuilder::new(&self.app_context, &self.project, task, node);
        builder.set_platform_manager(&self.platform_manager);
        builder.build(context).await.unwrap()
    }
}
