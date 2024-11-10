#![allow(dead_code)]

use moon_action::{ActionNode, RunTaskNode};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_platform::{PlatformManager, Runtime};
use moon_process::Command;
use moon_project::Project;
use moon_task::{Target, Task};
use moon_task_runner::command_builder::CommandBuilder;
use moon_task_runner::command_executor::CommandExecutor;
use moon_task_runner::output_archiver::OutputArchiver;
use moon_task_runner::output_hydrater::OutputHydrater;
use moon_task_runner::TaskRunner;
use moon_test_utils2::{
    generate_app_context_from_sandbox, generate_platform_manager_from_sandbox,
    generate_workspace_graph_from_sandbox, WorkspaceGraph,
};
use starbase_archive::Archiver;
use starbase_sandbox::{create_sandbox, Sandbox};
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
    pub workspace_graph: WorkspaceGraph,
}

impl TaskRunnerContainer {
    pub async fn new_for_project(fixture: &str, project_id: &str) -> Self {
        let sandbox = create_sandbox(fixture);
        let app_context = generate_app_context_from_sandbox(sandbox.path());
        let workspace_graph = generate_workspace_graph_from_sandbox(sandbox.path()).await;
        let project = workspace_graph.get_project(project_id).unwrap();
        let platform_manager = generate_platform_manager_from_sandbox(sandbox.path()).await;

        Self {
            sandbox,
            app_context,
            platform_manager,
            workspace_graph,
            project,
            project_id: project_id.to_owned(),
        }
    }

    pub async fn new_os(fixture: &str) -> Self {
        Self::new_for_project(fixture, if cfg!(windows) { "windows" } else { "unix" }).await
    }

    pub async fn new(fixture: &str) -> Self {
        Self::new_for_project(fixture, "project").await
    }

    pub fn create_archiver(&self, task_id: &str) -> OutputArchiver {
        let task = self.internal_get_task(task_id);

        OutputArchiver {
            app: &self.app_context,
            project_config: &self.project.config,
            task,
        }
    }

    pub fn create_hydrator(&self, task_id: &str) -> OutputHydrater {
        let task = self.internal_get_task(task_id);

        OutputHydrater {
            app: &self.app_context,
            task,
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
        let mut task = self.internal_get_task("base").to_owned();
        let mut node = create_node(&task);

        op(&mut task, &mut node);

        self.internal_create_command(&context, &task, &node).await
    }

    pub async fn create_command_executor(
        &self,
        task_id: &str,
        context: &ActionContext,
    ) -> CommandExecutor {
        let task = self.internal_get_task(task_id);
        let node = create_node(task);

        CommandExecutor::new(
            &self.app_context,
            &self.project,
            task,
            &node,
            self.internal_create_command(context, task, &node).await,
        )
    }

    pub fn create_runner(&self, task_id: &str) -> TaskRunner {
        let task = self.internal_get_task(task_id);

        let mut runner = TaskRunner::new(&self.app_context, &self.project, task).unwrap();
        runner.set_platform_manager(&self.platform_manager);
        runner
    }

    pub fn create_action_node(&self, task_id: &str) -> ActionNode {
        let task = self.internal_get_task(task_id);

        create_node(task)
    }

    pub fn pack_archive(&self, task_id: &str) -> PathBuf {
        let sandbox = &self.sandbox;
        let file = sandbox.path().join(".moon/cache/outputs/hash123.tar.gz");

        let out = format!(
            ".moon/cache/states/{}/{}/stdout.log",
            self.project_id, task_id,
        );

        let err = format!(
            ".moon/cache/states/{}/{}/stderr.log",
            self.project_id, task_id,
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

    fn internal_get_task(&self, task_id: &str) -> &Task {
        let target = Target::new(&self.project_id, task_id).unwrap();
        self.workspace_graph.tasks.get_unexpanded(&target).unwrap()
    }
}
