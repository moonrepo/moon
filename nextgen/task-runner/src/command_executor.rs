use crate::task_runner_error::TaskRunnerError;
use moon_action::{ActionNode, ActionStatus, Attempt};
use moon_action_context::ActionContext;
use moon_common::{is_ci, is_test_env};
use moon_config::TaskOutputStyle;
use moon_console::Console;
use moon_process::{output_to_error, Command, Output};
use moon_task::Task;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tokio::time::sleep;

fn is_ci_env() -> bool {
    is_ci() && !is_test_env()
}

/// Run the command as a child process and capture its output. If the process fails
/// and `retry_count` is greater than 0, attempt the process again in case it passes.
pub struct CommandExecutor<'task> {
    node: &'task ActionNode,
    task: &'task Task,

    command: Command,
    handle: Option<JoinHandle<()>>,

    attempts: Vec<Attempt>,
    attempt_index: u8,
    attempt_total: u8,

    // States
    interactive: bool,
    persistent: bool,
    stream: bool,
}

impl<'task> CommandExecutor<'task> {
    pub fn new(task: &'task Task, node: &'task ActionNode, command: Command) -> Self {
        Self {
            attempts: vec![],
            attempt_index: 0,
            attempt_total: task.options.retry_count + 1,
            interactive: node.is_interactive() || task.is_interactive(),
            persistent: node.is_persistent() || task.is_persistent(),
            stream: false,
            handle: None,
            node,
            task,
            command,
        }
    }

    pub async fn execute(
        mut self,
        context: &ActionContext,
        console: Arc<Console>,
    ) -> miette::Result<Vec<Attempt>> {
        self.prepate_state(context);

        // For long-running process, log a message on an interval to indicate it's still running
        self.start_monitoring();

        // Execute the command on a loop as an attempt for every retry count we have
        self.command.with_console(console);

        loop {
            let attempt = Attempt::new(self.attempt_index);
            let mut command = self.command.create_async();

            println!("RUNNING {}", self.task.target);

            if self.handle_execution(
                attempt,
                match (self.stream, self.interactive) {
                    (true, true) => command.exec_stream_output().await,
                    (true, false) => command.exec_stream_and_capture_output().await,
                    _ => command.exec_capture_output().await,
                },
            )? {
                break;
            }
        }

        println!("RAN {}", self.task.target);
        dbg!(&self.attempts);

        self.stop_monitoring();

        Ok(mem::take(&mut self.attempts))
    }

    fn handle_execution(
        &mut self,
        mut attempt: Attempt,
        execution: miette::Result<Output>,
    ) -> miette::Result<bool> {
        match execution {
            // Zero and non-zero exit codes
            Ok(mut output) => {
                attempt.finish_from_output(&mut output);

                self.attempts.push(attempt);

                // Successful execution, so break the loop
                if output.status.success() {
                    Ok(true)
                }
                // Unsuccessful execution (maybe flaky), attempt again
                else if self.attempt_index < self.attempt_total {
                    self.attempt_index += 1;

                    Ok(false)
                }
                // We've hit our max attempts, so error
                else {
                    Err(TaskRunnerError::RunFailed {
                        target: self.task.target.to_string(),
                        error: output_to_error(self.task.command.clone(), &output, false),
                    }
                    .into())
                }
            }

            // Process unexpectedly crashed
            Err(error) => {
                attempt.finish(ActionStatus::FailedAndAbort);

                self.attempts.push(attempt);

                Err(error)
            }
        }
    }

    fn start_monitoring(&mut self) {
        if self.persistent || self.interactive {
            return;
        }

        self.handle = Some(task::spawn(async move {
            let mut secs = 0;

            loop {
                sleep(Duration::from_secs(30)).await;
                secs += 30;

                println!("running for {}s", secs);

                // let _ = console_clone.out.print_checkpoint_with_comments(
                //     Checkpoint::RunStarted,
                //     &interval_target,
                //     [format!("running for {}s", secs)],
                // );
            }
        }));
    }

    fn stop_monitoring(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    fn prepate_state(&mut self, context: &ActionContext) {
        let is_primary = context.is_primary_target(&self.task.target);

        // When a task is configured as local (no caching), or the interactive flag is passed,
        // we don't "capture" stdout/stderr (which breaks stdin) and let it stream natively.
        if !self.task.options.cache && context.primary_targets.len() == 1 {
            self.interactive = true;
        }

        // When the primary target, always stream the output for a better developer experience.
        // However, transitive targets can opt into streaming as well.
        self.stream = if let Some(output_style) = &self.task.options.output_style {
            matches!(output_style, TaskOutputStyle::Stream)
        } else {
            is_primary || is_ci_env()
        };

        // Transitive targets may run concurrently, so differentiate them with a prefix.
        if self.stream && (is_ci_env() || !is_primary || context.primary_targets.len() > 1) {
            let prefix_max_width = context
                .primary_targets
                .iter()
                .map(|target| target.id.len())
                .max();

            self.command
                .set_prefix(&self.task.target.id, prefix_max_width);
        }
    }
}

impl<'task> Drop for CommandExecutor<'task> {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}
