use crate::reporter_ext::*;
use moon_action::{Action, ActionStatus, Attempt};
use moon_common::is_test_env;
use moon_config::TaskOutputStyle;
use moon_console::{Checkpoint, ConsoleBuffer, Reporter};
use moon_task::Task;
use moon_time as time;
use std::sync::Arc;

pub struct DefaultReporter {
    err: Arc<ConsoleBuffer>,
    out: Arc<ConsoleBuffer>,
}

impl DefaultReporter {
    pub fn print_task_checkpoint(
        &self,
        task: &Task,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        let mut comments = vec![];

        if task.is_no_op() {
            comments.push("no op".to_owned());
        } else if state.attempt_current > 1 {
            comments.push(format!(
                "attempt {}/{}",
                state.attempt_current, state.attempt_total
            ));
        }

        match attempt.status {
            ActionStatus::Cached => {
                comments.push("cached".into());
            }
            ActionStatus::CachedFromRemote => {
                comments.push("cached from remote".into());
            }
            ActionStatus::Skipped => {
                comments.push("skipped".into());
            }
            _ => {}
        };

        if let Some(duration) = attempt.duration {
            comments.push(time::elapsed(duration));
        }

        if let Some(hash) = &state.hash {
            // Do not include the hash while testing, as the hash
            // constantly changes and breaks our local snapshots
            if !is_test_env() && attempt.finished_at.is_some() {
                comments.push(hash[0..8].to_owned());
            }
        }

        self.out.print_checkpoint_with_comments(
            if attempt.has_failed() {
                Checkpoint::RunFailed
            } else if attempt.duration.is_none() {
                Checkpoint::RunStarted
            } else {
                Checkpoint::RunPassed
            },
            &task.target,
            comments,
        )?;

        Ok(())
    }

    pub fn print_attempt_output(
        &self,
        task: &Task,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        let print_stdout = || -> miette::Result<()> {
            if let Some(out) = &attempt.stdout {
                self.out.write_line(out)?;
            }

            Ok(())
        };

        let print_stderr = || -> miette::Result<()> {
            if let Some(out) = &attempt.stderr {
                self.err.write_line(out)?;
            }

            Ok(())
        };

        match task.options.output_style {
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
    fn inherit_streams(
        &mut self,
        err: Arc<ConsoleBuffer>,
        out: Arc<ConsoleBuffer>,
    ) -> miette::Result<()> {
        self.err = err;
        self.out = out;

        Ok(())
    }

    fn on_action_started(&mut self, _action: &Action) -> miette::Result<()> {
        Ok(())
    }

    fn on_action_completed(
        &mut self,
        _action: &Action,
        _error: Option<miette::Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_aborted(&mut self, _error: Option<miette::Report>) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_started(&mut self) -> miette::Result<()> {
        Ok(())
    }

    fn on_pipeline_completeed(&mut self, _error: Option<miette::Report>) -> miette::Result<()> {
        Ok(())
    }
}

impl TaskReporterExt for DefaultReporter {
    fn on_task_started(
        &mut self,
        task: &Task,
        attempt: &Attempt,
        state: &TaskReportState,
    ) -> miette::Result<()> {
        self.print_task_checkpoint(task, attempt, state)?;

        Ok(())
    }

    fn on_task_running(&mut self, _task: &Task, _state: &TaskReportState) -> miette::Result<()> {
        Ok(())
    }

    fn on_task_finished(
        &mut self,
        task: &Task,
        attempt: &Attempt,
        state: &TaskReportState,
        _error: Option<miette::Report>,
    ) -> miette::Result<()> {
        self.print_task_checkpoint(task, attempt, state)?;

        // Task was either cached or captured, so there was no output
        // sent to the console, so manually print the logs we have
        if attempt.is_cached() || !state.streamed_output {
            self.print_attempt_output(task, attempt, state)?;
        }

        Ok(())
    }
}
