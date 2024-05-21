use moon_action::{Action, ActionNode, ActionStatus, Attempt, AttemptType};
use moon_common::color::paint;
use moon_common::{color, is_test_env};
use moon_config::TaskOutputStyle;
use moon_console::*;
use moon_target::Target;
use moon_time as time;
use std::sync::Arc;

pub struct DefaultReporter {
    err: Arc<ConsoleBuffer>,
    out: Arc<ConsoleBuffer>,
}

impl Default for DefaultReporter {
    fn default() -> Self {
        Self {
            err: Arc::new(ConsoleBuffer::empty(ConsoleStream::Stderr)),
            out: Arc::new(ConsoleBuffer::empty(ConsoleStream::Stdout)),
        }
    }
}

impl DefaultReporter {
    fn get_status_meta_comment(
        &self,
        status: ActionStatus,
        fallback: impl Fn() -> Option<String>,
    ) -> Option<String> {
        match status {
            ActionStatus::Cached => Some("cached".into()),
            ActionStatus::CachedFromRemote => Some("cached from remote".into()),
            ActionStatus::Skipped => Some("skipped".into()),
            _ => fallback(),
        }
    }

    fn get_short_hash(&self, hash: &str) -> String {
        hash[0..8].to_owned()
    }

    fn print_task_checkpoint(
        &self,
        target: &Target,
        attempt: &Attempt,
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        let mut comments = vec![];

        match attempt.type_of {
            AttemptType::NoOperation => {
                comments.push("no op".into());
            }
            _ => {
                let status_comment = self.get_status_meta_comment(attempt.status, || {
                    if item.attempt_current > 1 {
                        Some(format!(
                            "attempt {}/{}",
                            item.attempt_current, item.attempt_total
                        ))
                    } else {
                        None
                    }
                });

                if let Some(comment) = status_comment {
                    comments.push(comment);
                }

                if let Some(duration) = attempt.duration {
                    if let Some(elapsed) = time::elapsed_opt(duration) {
                        comments.push(elapsed);
                    }
                }

                // Do not include the hash while testing, as the hash
                // constantly changes and breaks our local snapshots
                if !is_test_env() {
                    if let Some(hash) = &item.hash {
                        comments.push(self.get_short_hash(hash));
                    }
                }
            }
        };

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
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        let print_stdout = || -> miette::Result<()> {
            if let Some(execution) = &attempt.execution {
                if let Some(out) = &execution.stdout {
                    self.out.write_line(out.trim())?;
                }
            }

            Ok(())
        };

        let print_stderr = || -> miette::Result<()> {
            if let Some(execution) = &attempt.execution {
                if let Some(out) = &execution.stderr {
                    self.err.write_line(out.trim())?;
                }
            }

            Ok(())
        };

        match item.output_style {
            // Only show output on failure
            Some(TaskOutputStyle::BufferOnlyFailure) => {
                if attempt.has_failed() {
                    print_stdout()?;
                    print_stderr()?;
                }
            }
            // Only show the hash
            Some(TaskOutputStyle::Hash) => {
                if let Some(hash) = &item.hash {
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

    fn print_pipeline_failures(&self, actions: &[Action]) -> miette::Result<()> {
        for action in actions {
            if !action.has_failed() {
                continue;
            }

            if let Some(attempts) = &action.attempts {
                if let Some(attempt) = Attempt::get_last_failed_execution(attempts) {
                    let mut has_stdout = false;

                    if let Some(execution) = &attempt.execution {
                        if let Some(stdout) = &execution.stdout {
                            if !stdout.is_empty() {
                                has_stdout = true;
                                self.out.write_line(stdout.trim())?;
                            }
                        }

                        if let Some(stderr) = &execution.stderr {
                            if has_stdout {
                                self.out.write_newline()?;
                            }

                            if !stderr.is_empty() {
                                self.out.write_line(stderr.trim())?;
                            }
                        }
                    }
                }
            }

            self.out.print_checkpoint(
                Checkpoint::RunFailed,
                match &*action.node {
                    ActionNode::RunTask(inner) => inner.target.as_str(),
                    _ => &action.label,
                },
            )?;

            self.out.write_newline()?;
        }

        Ok(())
    }

    fn print_pipeline_stats(
        &self,
        actions: &[Action],
        item: &PipelineReportItem,
    ) -> miette::Result<()> {
        let mut passed_count = 0;
        let mut cached_count = 0;
        let mut failed_count = 0;
        let mut invalid_count = 0;
        let mut skipped_count = 0;

        for action in actions {
            if !item.summarize && !matches!(*action.node, ActionNode::RunTask { .. }) {
                continue;
            }

            match action.status {
                ActionStatus::Cached | ActionStatus::CachedFromRemote => {
                    cached_count += 1;
                    passed_count += 1;
                }
                ActionStatus::Passed => {
                    passed_count += 1;
                }
                ActionStatus::Failed | ActionStatus::FailedAndAbort => {
                    failed_count += 1;
                }
                ActionStatus::Invalid => {
                    invalid_count += 1;
                }
                ActionStatus::Skipped => {
                    skipped_count += 1;
                }
                ActionStatus::Running => {}
            }
        }

        let mut counts_message = vec![];

        if passed_count > 0 {
            if cached_count > 0 {
                counts_message.push(format!(
                    "{} {}",
                    color::success(format!("{passed_count} completed")),
                    color::label(format!("({cached_count} cached)"))
                ));
            } else {
                counts_message.push(color::success(format!("{passed_count} completed")));
            }
        }

        if failed_count > 0 {
            counts_message.push(color::failure(format!("{failed_count} failed")));
        }

        if invalid_count > 0 {
            counts_message.push(color::invalid(format!("{invalid_count} invalid")));
        }

        if skipped_count > 0 {
            counts_message.push(color::muted_light(format!("{skipped_count} skipped")));
        }

        let counts_message = counts_message.join(&color::muted(", "));
        let mut elapsed_time = time::elapsed(item.duration.unwrap_or_default());

        if passed_count == cached_count && failed_count == 0 {
            elapsed_time = format!("{} {}", elapsed_time, label_to_the_moon());
        }

        if item.summarize {
            self.out.print_entry("Actions", &counts_message)?;
            self.out.print_entry("   Time", &elapsed_time)?;
        } else {
            self.out.print_entry("Tasks", &counts_message)?;
            self.out.print_entry(" Time", &elapsed_time)?;
        }

        Ok(())
    }

    fn print_pipeline_summary(&self, actions: &[Action]) -> miette::Result<()> {
        for action in actions {
            let status = match action.status {
                ActionStatus::Passed => color::success("pass"),
                ActionStatus::Cached | ActionStatus::CachedFromRemote => color::label("pass"),
                ActionStatus::Failed | ActionStatus::FailedAndAbort => color::failure("fail"),
                ActionStatus::Invalid => color::invalid("warn"),
                ActionStatus::Skipped => color::muted_light("skip"),
                ActionStatus::Running => color::muted_light("oops"),
            };

            let mut comments: Vec<String> = vec![];

            if let Some(status_comment) = self.get_status_meta_comment(action.status, || None) {
                comments.push(status_comment);
            }

            if let Some(duration) = action.duration {
                if let Some(elapsed) = time::elapsed_opt(duration) {
                    comments.push(elapsed);
                }
            }

            if let Some(hash) = &action.hash {
                comments.push(self.get_short_hash(hash));
            }

            self.out.write_line(format!(
                "{} {} {}",
                status,
                action.label,
                self.out.format_comments(comments),
            ))?;
        }

        Ok(())
    }
}

impl Reporter for DefaultReporter {
    fn inherit_streams(&mut self, err: Arc<ConsoleBuffer>, out: Arc<ConsoleBuffer>) {
        self.err = err;
        self.out = out;
    }

    fn on_pipeline_completed(
        &self,
        actions: &[Action],
        item: &PipelineReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        if actions.is_empty() || self.out.is_quiet() {
            return Ok(());
        }

        // If no summary, only show stats. This is typically for local!
        if !item.summarize {
            self.out.write_newline()?;
            self.print_pipeline_stats(actions, item)?;
            self.out.write_newline()?;

            return Ok(());
        }

        // Otherwise, show all the information we can.
        if actions.iter().any(|action| action.has_failed()) {
            self.out.print_header("Review")?;
            self.print_pipeline_failures(actions)?;
        }

        self.out.print_header("Summary")?;
        self.print_pipeline_summary(actions)?;

        self.out.print_header("Stats")?;
        self.print_pipeline_stats(actions, item)?;

        self.out.write_newline()?;

        Ok(())
    }

    // Print a checkpoint when a task execution starts, for each attempt
    fn on_task_started(
        &self,
        target: &Target,
        attempt: &Attempt,
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        self.print_task_checkpoint(target, attempt, item)?;

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

    // When an attempt has finished, print the output if captured
    fn on_task_finished(
        &self,
        _target: &Target,
        attempt: &Attempt,
        item: &TaskReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        // Task output was captured, so there was no output
        // sent to the console, so manually print the logs we have!
        if !item.output_streamed && attempt.has_output() {
            self.print_attempt_output(attempt, item)?;
        }

        Ok(())
    }

    // When all attempts have completed, print the final checkpoint
    fn on_task_completed(
        &self,
        target: &Target,
        attempts: &[Attempt],
        item: &TaskReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        if let Some(attempt) = Attempt::get_last_execution(attempts) {
            // If cached, the finished event above is not fired,
            // so handle printing the captured logs here!
            if attempt.is_cached() && attempt.has_output() {
                self.out.write_newline()?;
                self.print_attempt_output(attempt, item)?;
            }

            // Then print the success checkpoint. The success
            // checkpoint should always appear after the output,
            // and "contain" it within the start checkpoint!
            self.print_task_checkpoint(target, attempt, item)?;
        } else if let Some(attempt) = attempts.last() {
            self.print_task_checkpoint(target, attempt, item)?;
        }

        Ok(())
    }
}

fn label_to_the_moon() -> String {
    [
        paint(55, "❯"),
        paint(56, "❯❯"),
        paint(57, "❯ t"),
        paint(63, "o t"),
        paint(69, "he "),
        paint(75, "mo"),
        paint(81, "on"),
    ]
    .into_iter()
    .collect::<Vec<_>>()
    .join("")
}
