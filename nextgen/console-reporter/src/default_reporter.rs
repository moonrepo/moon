use moon_action::{ActionStatus, Attempt, AttemptType};
use moon_common::is_test_env;
use moon_config::TaskOutputStyle;
use moon_console::{Checkpoint, ConsoleBuffer, ConsoleStream, Reporter, TaskReportState};
use moon_target::Target;
use moon_time as time;
use std::sync::Arc;

pub struct DefaultReporter {
    err: Arc<ConsoleBuffer>,
    out: Arc<ConsoleBuffer>,
}

impl DefaultReporter {
    pub fn new() -> Self {
        Self {
            err: Arc::new(ConsoleBuffer::empty(ConsoleStream::Stderr)),
            out: Arc::new(ConsoleBuffer::empty(ConsoleStream::Stdout)),
        }
    }

    pub fn print_task_checkpoint(
        &self,
        target: &Target,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        let mut comments = vec![];

        match attempt.type_of {
            AttemptType::NoOperation => {
                comments.push("no op".into());
            }
            _ => match attempt.status {
                ActionStatus::Cached => {
                    comments.push("cached".into());
                }
                ActionStatus::CachedFromRemote => {
                    comments.push("cached from remote".into());
                }
                ActionStatus::Skipped => {
                    comments.push("skipped".into());
                }
                _ => {
                    if state.attempt_current > 1 {
                        comments.push(format!(
                            "attempt {}/{}",
                            state.attempt_current, state.attempt_total
                        ));
                    }
                }
            },
        };

        if let Some(duration) = attempt.duration {
            if let Some(elapsed) = time::elapsed_opt(duration) {
                comments.push(elapsed);
            }
        }

        // Do not include the hash while testing, as the hash
        // constantly changes and breaks our local snapshots
        if !is_test_env() {
            if let Some(hash) = &state.hash {
                comments.push(hash[0..8].to_owned());
            }
        }

        self.out.print_checkpoint_with_comments(
            if attempt.has_failed() {
                Checkpoint::RunFailed
            } else if attempt.has_passed() {
                Checkpoint::RunPassed
            } else {
                Checkpoint::RunStarted
            },
            target,
            comments,
        )?;

        Ok(())
    }

    pub fn print_attempt_output(
        &self,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        let print_stdout = || -> miette::Result<()> {
            if let Some(out) = &attempt.stdout {
                self.out.write_line(out.as_bytes())?;
            }

            Ok(())
        };

        let print_stderr = || -> miette::Result<()> {
            if let Some(out) = &attempt.stderr {
                self.err.write_line(out.as_bytes())?;
            }

            Ok(())
        };

        match state.output_style {
            // Only show output on failure
            Some(TaskOutputStyle::BufferOnlyFailure) => {
                if attempt.has_failed() {
                    print_stdout()?;
                    print_stderr()?;
                }
            }
            // Only show the hash
            Some(TaskOutputStyle::Hash) => {
                if let Some(hash) = &state.hash {
                    // Print to stderr so it can be captured
                    self.err.write_line(hash)?;
                }
            }
            // Show nothing
            Some(TaskOutputStyle::None) => {}
            // Show output on both success and failure
            _ => {
                print_stdout()?;
                print_stderr()?;
            }
        };

        Ok(())
    }
}

impl Reporter for DefaultReporter {
    fn inherit_streams(&mut self, err: Arc<ConsoleBuffer>, out: Arc<ConsoleBuffer>) {
        self.err = err;
        self.out = out;
    }

    // Print a checkpoint when a task execution starts, for each attemp
    fn on_task_started(
        &self,
        target: &Target,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        self.print_task_checkpoint(target, attempt, state)?;

        Ok(())
    }

    // If the task has been running for a long time, print a checkpoint
    fn on_task_running(&self, target: &Target, secs: u32) -> miette::Result<()> {
        self.out.print_checkpoint_with_comments(
            Checkpoint::RunStarted,
            target,
            [format!("running for {}s", secs)],
        )?;

        Ok(())
    }

    // When an attempt has finished, print the output and checkpoint
    fn on_task_finished(
        &self,
        target: &Target,
        attempt: &Attempt,
        state: &TaskReportState,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        // If successful, print the checkpoint so that the header appears
        // above the stderr/out output below
        if attempt.has_passed() {
            self.print_task_checkpoint(target, attempt, state)?;
        }

        // Task was either cached or captured, so there was no output
        // sent to the console, so manually print the logs we have
        if attempt.is_cached() || !state.output_streamed {
            self.print_attempt_output(attempt, state)?;
        }

        Ok(())
    }

    fn on_task_completed(
        &self,
        target: &Target,
        attempts: &[Attempt],
        state: &TaskReportState,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        if let Some(attempt) = attempts.last() {
            self.print_task_checkpoint(target, attempt, state)?;
        }

        Ok(())
    }
}
