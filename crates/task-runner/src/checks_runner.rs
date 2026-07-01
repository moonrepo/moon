use crate::check_executor::{CheckExecuteResult, CheckExecutor};
use crate::command_builder::CommandBuilder;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_project::Project;
use moon_task::{Task, TaskCheckType};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

pub struct ChecksRunner<'task> {
    app_context: &'task Arc<AppContext>,
    project: &'task Arc<Project>,
    task: &'task Arc<Task>,
}

impl<'task> ChecksRunner<'task> {
    pub fn new(
        app_context: &'task Arc<AppContext>,
        project: &'task Arc<Project>,
        task: &'task Arc<Task>,
    ) -> miette::Result<Self> {
        Ok(Self {
            project,
            task,
            app_context,
        })
    }

    #[instrument(skip(self))]
    pub async fn execute(
        self,
        check_types: Vec<TaskCheckType>,
    ) -> miette::Result<Vec<CheckExecuteResult>> {
        let checks = self
            .task
            .checks
            .iter()
            .filter(|check| check_types.contains(&check.get_type()))
            .collect::<Vec<_>>();

        if checks.is_empty() {
            return Ok(vec![]);
        }

        debug!(
            task_target = self.task.target.as_str(),
            check_types = ?check_types,
            checks_count = checks.len(),
            "Building and executing task checks"
        );

        // Execute the checks in parallel
        let mut set = JoinSet::new();

        for check in checks {
            let command = CommandBuilder::new(self.app_context, self.project, self.task)
                .build_check(check)
                .await?;

            let executor = CheckExecutor::new(
                self.app_context.clone(),
                self.project.clone(),
                self.task.clone(),
                command,
            );

            let check = check.to_owned();

            set.spawn(async move { executor.execute(check).await });
        }

        let mut results = vec![];

        while let Some(result) = set.join_next().await {
            results.push(result.into_diagnostic()??);
        }

        Ok(results)
    }
}
