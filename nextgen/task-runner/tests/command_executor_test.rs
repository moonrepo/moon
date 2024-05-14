mod utils;

use moon_action::{ActionNode, ActionStatus, AttemptType};
use moon_action_context::ActionContext;
use moon_console::Console;
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_task_runner::command_builder::CommandBuilder;
use moon_task_runner::command_executor::CommandExecutor;
use moon_test_utils2::{
    generate_platform_manager_from_sandbox, generate_project_graph_from_sandbox, ProjectGraph,
};
use moon_workspace::Workspace;
use starbase_sandbox::{create_sandbox, Sandbox};
use std::sync::Arc;
use utils::*;

pub struct CommandExecutorContainer {
    pub project_graph: ProjectGraph,
    pub project: Arc<Project>,
    pub workspace: Workspace,
}

impl CommandExecutorContainer {
    pub fn modify_workspace(&mut self, mut op: impl FnMut(&mut Workspace)) {
        op(&mut self.workspace);
    }

    pub async fn build_command(
        &self,
        task: &Task,
        node: &ActionNode,
        context: &ActionContext,
    ) -> Command {
        let platform = generate_platform_manager_from_sandbox(&self.workspace.root).await;

        let mut builder = CommandBuilder::new(&self.workspace, &self.project, task, node);
        builder.set_platform_manager(&platform);
        builder.build(context).await.unwrap()
    }

    pub async fn build(&self, task_id: &str, context: &ActionContext) -> CommandExecutor {
        let task = self.project.get_task(task_id).unwrap();
        let node = create_node(task);

        CommandExecutor::new(
            &self.workspace,
            &self.project,
            task,
            &node,
            self.build_command(task, &node, context).await,
        )
    }
}

async fn generate_container() -> (Sandbox, CommandExecutorContainer) {
    let sandbox = create_sandbox("executor");
    let workspace = create_workspace(sandbox.path());
    let project_graph = generate_project_graph_from_sandbox(sandbox.path()).await;
    let project = project_graph
        .get(if cfg!(windows) { "windows" } else { "unix" })
        .unwrap();

    (
        sandbox,
        CommandExecutorContainer {
            project,
            project_graph,
            workspace,
        },
    )
}

mod command_executor {
    use super::*;

    #[tokio::test]
    async fn returns_attempt_on_success() {
        let (_sandbox, container) = generate_container().await;
        let context = ActionContext::default();
        let console = Console::new_testing();

        let result = container
            .build("success", &context)
            .await
            .execute("hash123", &context, &console)
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert_eq!(result.state.hash.unwrap(), "hash123");
        assert_eq!(result.state.attempt_current, 1);
        assert_eq!(result.state.attempt_total, 1);

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let exec = attempt.execution.as_ref().unwrap();

        assert_eq!(attempt.status, ActionStatus::Passed);
        assert_eq!(attempt.type_of, AttemptType::TaskExecution);
        assert_eq!(exec.exit_code.unwrap(), 0);
        assert_eq!(exec.stdout.as_ref().unwrap().trim(), "test");
    }

    #[tokio::test]
    async fn returns_attempt_on_failure() {
        let (_sandbox, container) = generate_container().await;
        let context = ActionContext::default();
        let console = Console::new_testing();

        let result = container
            .build("failure", &context)
            .await
            .execute("hash123", &context, &console)
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert_eq!(result.state.hash.unwrap(), "hash123");
        assert_eq!(result.state.attempt_current, 1);
        assert_eq!(result.state.attempt_total, 1);

        // Check attempt
        assert_eq!(result.attempts.len(), 1);

        let attempt = result.attempts.first().unwrap();
        let exec = attempt.execution.as_ref().unwrap();

        assert_eq!(attempt.status, ActionStatus::Failed);
        assert_eq!(attempt.type_of, AttemptType::TaskExecution);
        assert_eq!(exec.exit_code.unwrap(), 1);
    }

    #[tokio::test]
    async fn returns_attempts_for_each_retry() {
        let (_sandbox, container) = generate_container().await;
        let context = ActionContext::default();
        let console = Console::new_testing();

        let result = container
            .build("retry", &context)
            .await
            .execute("", &context, &console)
            .await
            .unwrap();

        // Check state
        assert!(result.error.is_none());
        assert!(result.state.hash.is_none());
        assert_eq!(result.state.attempt_current, 4);
        assert_eq!(result.state.attempt_total, 4);

        // Check attempt
        assert_eq!(result.attempts.len(), 4);

        for i in 0..4 {
            let attempt = &result.attempts[i];
            let exec = attempt.execution.as_ref().unwrap();

            assert_eq!(attempt.status, ActionStatus::Failed);
            assert_eq!(attempt.type_of, AttemptType::TaskExecution);
            assert_eq!(exec.exit_code.unwrap(), 1);
        }
    }
}
