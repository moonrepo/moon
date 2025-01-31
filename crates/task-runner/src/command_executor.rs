use moon_action::{ActionNode, ActionStatus, Operation, OperationList};
use moon_action_context::{ActionContext, TargetState};
use moon_app_context::AppContext;
use moon_common::{is_ci, is_test_env};
use moon_config::TaskOutputStyle;
use moon_console::TaskReportItem;
use moon_process::{args::join_args, Command, CommandLine};
use moon_project::Project;
use moon_task::Task;
use std::process::Output;
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument};

fn is_ci_env() -> bool {
    is_ci() && !is_test_env()
}

#[derive(Debug)]
pub struct CommandExecuteResult {
    pub attempts: OperationList,
    pub error: Option<miette::Report>,
    pub run_state: TargetState,
}

/// Run the command as a child process and capture its output. If the process fails
/// and `retry_count` is greater than 0, attempt the process again in case it passes.
pub struct CommandExecutor<'task> {
    app: &'task AppContext,
    task: &'task Task,
    project: &'task Project,

    command: Command,
    handle: Option<JoinHandle<()>>,

    attempts: OperationList,
    attempt_index: u8,
    attempt_total: u8,

    // States
    interactive: bool,
    persistent: bool,
    stream: bool,
}

impl<'task> CommandExecutor<'task> {
    pub fn new(
        app: &'task AppContext,
        project: &'task Project,
        task: &'task Task,
        node: &ActionNode,
        mut command: Command,
    ) -> Self {
        command.with_console(app.console.clone());

        Self {
            attempts: OperationList::default(),
            attempt_index: 1,
            attempt_total: task.options.retry_count + 1,
            interactive: node.is_interactive() || task.is_interactive(),
            persistent: node.is_persistent() || task.is_persistent(),
            stream: false,
            handle: None,
            app,
            project,
            task,
            command,
        }
    }

    #[instrument(skip(self, context))]
    pub async fn execute(
        mut self,
        context: &ActionContext,
        report_item: &mut TaskReportItem,
    ) -> miette::Result<CommandExecuteResult> {
        // Prepare state for the executor, and each attempt
        let mut run_state = TargetState::Failed;

        self.prepare_state(context, report_item);

        // For long-running process, log a message on an interval to indicate it's still running
        self.monitor_running_status();

        // Execute the command on a loop as an attempt for every retry count we have
        let command_line = self.get_command_line(context);

        let execution_error: Option<miette::Report> = loop {
            let mut attempt = Operation::task_execution(&command_line);
            report_item.attempt_current = self.attempt_index;

            debug!(
                task_target = self.task.target.as_str(),
                command = self.command.bin.to_str(),
                "Running task (attempt {} of {})",
                self.attempt_index,
                self.attempt_total
            );

            self.app
                .console
                .reporter
                .on_task_started(&self.task.target, &attempt, report_item)?;

            self.print_command_line(&command_line)?;

            // Attempt to execute command
            async fn execute_command(
                command: &mut Command,
                stream: bool,
                interactive: bool,
            ) -> miette::Result<Output> {
                match (stream, interactive) {
                    (true, true) | (false, true) => command.exec_stream_output().await,
                    (true, false) => command.exec_stream_and_capture_output().await,
                    _ => command.exec_capture_output().await,
                }
            }

            let timeout_token = CancellationToken::new();
            let timeout_handle =
                self.monitor_timeout(self.task.options.timeout, timeout_token.clone());

            let attempt_result = tokio::select! {
                // Run conditions in order!
                biased;

                // Cancel if we have timed out
                _ = timeout_token.cancelled() => {
                    Ok(None)
                }

                // Or run the job to completion
                result = execute_command(
                    &mut self.command,
                    self.stream,
                    self.interactive,
                ) => result.map(Some),
            };

            // Cleanup before sending the result
            if let Some(handle) = timeout_handle {
                handle.abort();
            }

            // Handle the execution result
            match attempt_result {
                // Zero and non-zero exit codes
                Ok(maybe_output) => {
                    let mut is_success = false;

                    if let Some(output) = maybe_output {
                        is_success = output.status.success();

                        debug!(
                            task_target = self.task.target.as_str(),
                            command = self.command.bin.to_str(),
                            exit_code = output.status.code(),
                            "Ran task, checking conditions",
                        );

                        attempt.finish_from_output(output);
                    } else {
                        debug!(
                            task_target = self.task.target.as_str(),
                            command = self.command.bin.to_str(),
                            "Task timed out",
                        );

                        attempt.finish(ActionStatus::TimedOut);
                    }

                    self.app.console.reporter.on_task_finished(
                        &self.task.target,
                        &attempt,
                        report_item,
                        None,
                    )?;

                    self.attempts.push(attempt);

                    // Successful execution, so break the loop
                    if is_success {
                        debug!(
                            task_target = self.task.target.as_str(),
                            "Task was successful, proceeding to next step",
                        );

                        run_state = TargetState::from_hash(report_item.hash.as_deref());
                        break None;
                    }
                    // Unsuccessful execution (maybe flaky), attempt again
                    else if self.attempt_index < self.attempt_total {
                        debug!(
                            task_target = self.task.target.as_str(),
                            "Task was unsuccessful, attempting again",
                        );

                        self.attempt_index += 1;
                        continue;
                    }
                    // We've hit our max attempts, so break
                    else {
                        debug!(
                            task_target = self.task.target.as_str(),
                            "Task was unsuccessful, failing as we hit our max attempts",
                        );

                        break None;
                    }
                }

                // Process unexpectedly crashed
                Err(error) => {
                    debug!(
                        task_target = self.task.target.as_str(),
                        command = self.command.bin.to_str(),
                        "Failed to run task, an unexpected error occurred",
                    );

                    attempt.finish(ActionStatus::Failed);

                    self.app.console.reporter.on_task_finished(
                        &self.task.target,
                        &attempt,
                        report_item,
                        Some(&error),
                    )?;

                    self.attempts.push(attempt);

                    break Some(error);
                }
            }
        };

        self.stop_monitoring();

        Ok(CommandExecuteResult {
            attempts: self.attempts.take(),
            error: execution_error,
            run_state,
        })
    }

    fn monitor_running_status(&mut self) {
        if self.persistent {
            return;
        }

        let console = self.app.console.clone();
        let target = self.task.target.clone();

        self.handle = Some(task::spawn(async move {
            let mut secs: u32 = 0;

            loop {
                sleep(Duration::from_secs(30)).await;
                secs += 30;

                let _ = console.reporter.on_task_running(&target, secs);
            }
        }));
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

    fn stop_monitoring(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    fn prepare_state(&mut self, context: &ActionContext, report_item: &mut TaskReportItem) {
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

        // If only a single persistent task is being ran, we should not prefix the output.
        if context.primary_targets.len() == 1
            && is_primary
            && (self.task.is_persistent() || self.task.deps.is_empty())
        {
            report_item.output_prefix = None;
        }

        if let Some(prefix) = &report_item.output_prefix {
            self.command.set_prefix(prefix);
        }

        report_item.attempt_current = self.attempt_index;
        report_item.attempt_total = self.attempt_total;
        report_item.output_streamed = self.stream;
    }

    fn get_command_line(&self, context: &ActionContext) -> String {
        if self.task.script.is_some() {
            self.task.get_command_line()
        } else {
            let mut args = vec![&self.task.command];
            args.extend(&self.task.args);

            if context.should_inherit_args(&self.task.target) {
                args.extend(&context.passthrough_args);
            }

            join_args(args)
        }
    }

    // We don't use `Command::print_command` because we need to explicitly
    // control the workspace root and working directory!
    fn print_command_line(&self, command_line: &str) -> miette::Result<()> {
        if !self.app.workspace_config.runner.log_running_command {
            return Ok(());
        }

        let workspace_root = &self.app.workspace_root;
        let working_dir = if self.task.options.run_from_workspace_root {
            &self.app.workspace_root
        } else {
            &self.project.root
        };

        self.app.console.out.write_line(CommandLine::format(
            command_line,
            workspace_root,
            working_dir,
        ))?;

        Ok(())
    }
}

impl Drop for CommandExecutor<'_> {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}
