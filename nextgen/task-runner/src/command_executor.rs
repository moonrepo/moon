use moon_action::{ActionNode, ActionStatus, Attempt, AttemptType};
use moon_action_context::ActionContext;
use moon_common::{color, is_ci, is_test_env};
use moon_config::TaskOutputStyle;
use moon_console::{Console, TaskReportState};
use moon_process::args::join_args;
use moon_process::Command;
use moon_project::Project;
use moon_task::Task;
use moon_workspace::Workspace;
use std::mem;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{self, JoinHandle};
use tokio::time::sleep;

fn is_ci_env() -> bool {
    is_ci() && !is_test_env()
}

#[derive(Debug)]
pub struct CommandExecuteResult {
    pub attempts: Vec<Attempt>,
    pub error: Option<miette::Report>,
    pub state: TaskReportState,
}

/// Run the command as a child process and capture its output. If the process fails
/// and `retry_count` is greater than 0, attempt the process again in case it passes.
pub struct CommandExecutor<'task> {
    task: &'task Task,
    project: &'task Project,
    workspace: &'task Workspace,

    command: Command,
    console: Arc<Console>,
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
    pub fn new(
        workspace: &'task Workspace,
        project: &'task Project,
        task: &'task Task,
        node: &ActionNode,
        console: Arc<Console>,
        mut command: Command,
    ) -> Self {
        command.with_console(console.clone());

        Self {
            attempts: vec![],
            attempt_index: 1,
            attempt_total: task.options.retry_count + 1,
            interactive: node.is_interactive() || task.is_interactive(),
            persistent: node.is_persistent() || task.is_persistent(),
            stream: false,
            handle: None,
            workspace,
            project,
            task,
            command,
            console,
        }
    }

    pub async fn execute(
        mut self,
        context: &ActionContext,
        hash: Option<&str>,
    ) -> miette::Result<CommandExecuteResult> {
        // Prepare state for the executor, and each attempt
        let mut state = self.prepate_state(context);

        // Hash is empty if cache is disabled
        state.hash = hash.map(|h| h.to_string());

        // For long-running process, log a message on an interval to indicate it's still running
        self.start_monitoring();

        // Execute the command on a loop as an attempt for every retry count we have
        let execution_error: Option<miette::Report> = loop {
            let mut attempt = Attempt::new(AttemptType::TaskExecution);
            state.attempt_current = self.attempt_index;

            self.console
                .reporter
                .on_task_started(&self.task.target, &attempt, &state)?;

            self.print_command(context)?;

            // Attempt to execute command
            let mut command = self.command.create_async();

            let attempt_result = match (self.stream, self.interactive) {
                (true, true) | (false, true) => command.exec_stream_output().await,
                (true, false) => command.exec_stream_and_capture_output().await,
                _ => command.exec_capture_output().await,
            };

            // Handle the execution result
            match attempt_result {
                // Zero and non-zero exit codes
                Ok(mut output) => {
                    attempt.finish_from_output(&mut output);

                    self.console.reporter.on_task_finished(
                        &self.task.target,
                        &attempt,
                        &state,
                        None,
                    )?;

                    self.attempts.push(attempt);

                    // Successful execution, so break the loop
                    if output.status.success() {
                        break None;
                    }
                    // Unsuccessful execution (maybe flaky), attempt again
                    else if self.attempt_index < self.attempt_total {
                        self.attempt_index += 1;
                        continue;
                    }
                    // We've hit our max attempts, so break
                    else {
                        break None;
                    }
                }

                // Process unexpectedly crashed
                Err(error) => {
                    attempt.finish(ActionStatus::Failed);

                    self.console.reporter.on_task_finished(
                        &self.task.target,
                        &attempt,
                        &state,
                        Some(&error),
                    )?;

                    self.attempts.push(attempt);

                    break Some(error);
                }
            }
        };

        self.stop_monitoring();

        Ok(CommandExecuteResult {
            attempts: mem::take(&mut self.attempts),
            error: execution_error,
            state,
        })
    }

    fn start_monitoring(&mut self) {
        if self.persistent || self.interactive {
            return;
        }

        let console = self.console.clone();
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

    fn stop_monitoring(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    fn prepate_state(&mut self, context: &ActionContext) -> TaskReportState {
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

        TaskReportState {
            attempt_current: self.attempt_index,
            attempt_total: self.attempt_total,
            hash: None,
            output_streamed: self.stream,
            output_style: self.task.options.output_style,
        }
    }

    fn print_command(&self, context: &ActionContext) -> miette::Result<()> {
        if !self.workspace.config.runner.log_running_command {
            return Ok(());
        }

        let task = &self.task;

        let mut args = vec![&task.command];
        args.extend(&task.args);

        if context.should_inherit_args(&task.target) {
            args.extend(&context.passthrough_args);
        }

        let command_line = join_args(args);

        let message = color::muted_light(self.command.inspect().format_command(
            &command_line,
            &self.workspace.root,
            Some(if task.options.run_from_workspace_root {
                &self.workspace.root
            } else {
                &self.project.root
            }),
        ));

        self.console.out.write_line(message)?;

        Ok(())
    }
}

impl<'task> Drop for CommandExecutor<'task> {
    fn drop(&mut self) {
        self.stop_monitoring();
    }
}
