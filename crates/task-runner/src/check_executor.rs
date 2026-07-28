use moon_action::{ActionStatus, Operation};
use moon_app_context::AppContext;
use moon_process::{Command, Output};
use moon_task::{Task, TaskCheck};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct CheckExecuteResult {
    pub attempt: Operation,
    pub check: TaskCheck,
    pub error: Option<miette::Report>,
    pub output: Option<Output>,
}

pub struct CheckExecutor {
    task: Arc<Task>,
    command: Command,
}

impl CheckExecutor {
    pub fn new(app: Arc<AppContext>, task: Arc<Task>, mut command: Command) -> Self {
        command.set_console(app.console.clone());

        Self { task, command }
    }

    #[instrument(skip(self))]
    pub async fn execute(mut self, check: TaskCheck) -> miette::Result<CheckExecuteResult> {
        let command_line = self.command.get_command_line(false, false);
        let mut attempt = Operation::process_execution(&command_line);

        debug!(
            task_target = self.task.target.as_str(),
            command = self.command.get_bin_name(),
            "Running task check"
        );

        let timeout_token = CancellationToken::new();
        let timeout_handle = self.monitor_timeout(self.task.options.timeout, timeout_token.clone());

        let attempt_result = tokio::select! {
            // Run conditions in order!
            biased;

            // Cancel if we have timed out
            _ = timeout_token.cancelled() => {
                Ok(None)
            }

            // Or run the job to completion
            result = self.command.exec_capture_output() => result.map(Some),
        };

        // Cleanup before sending the result
        if let Some(handle) = timeout_handle {
            handle.abort();
        }

        match attempt_result {
            // Zero and non-zero exit codes
            Ok(maybe_output) => {
                let mut is_success = false;

                if let Some(output) = &maybe_output {
                    is_success = output.success();

                    debug!(
                        task_target = self.task.target.as_str(),
                        command = self.command.get_bin_name(),
                        exit_code = output.code(),
                        "Ran task check",
                    );

                    attempt.finish_from_output(
                        output.status(),
                        output.stdout.clone(),
                        output.stderr.clone(),
                    );
                } else {
                    debug!(
                        task_target = self.task.target.as_str(),
                        command = self.command.get_bin_name(),
                        "Task check timed out",
                    );

                    attempt.finish(ActionStatus::TimedOut);
                }

                if is_success {
                    debug!(
                        task_target = self.task.target.as_str(),
                        "Task check was successful, proceeding to next step",
                    );
                } else {
                    debug!(
                        task_target = self.task.target.as_str(),
                        "Task check was unsuccessful",
                    );
                }

                Ok(CheckExecuteResult {
                    attempt,
                    check,
                    error: None,
                    output: maybe_output,
                })
            }

            // Process unexpectedly crashed
            Err(error) => {
                debug!(
                    task_target = self.task.target.as_str(),
                    command = self.command.get_bin_name(),
                    "Failed to run task check, an unexpected error occurred",
                );

                attempt.finish(ActionStatus::Failed);

                Ok(CheckExecuteResult {
                    attempt,
                    check,
                    error: Some(error),
                    output: None,
                })
            }
        }
    }

    fn monitor_timeout(
        &self,
        duration: Option<u64>,
        timeout_token: CancellationToken,
    ) -> Option<JoinHandle<()>> {
        duration.map(|duration| {
            tokio::spawn(async move {
                if timeout(
                    Duration::from_secs(duration),
                    sleep(Duration::from_secs(86400)), // 1 day
                )
                .await
                .is_err()
                {
                    timeout_token.cancel();
                }
            })
        })
    }
}
