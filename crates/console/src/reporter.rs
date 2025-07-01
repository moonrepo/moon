use moon_action::{
    Action, ActionNode, ActionPipelineStatus, ActionStatus, Operation, OperationList,
};
use moon_common::{color, is_test_env};
use moon_config::TaskOutputStyle;
use moon_target::Target;
use moon_time as time;
use starbase_console::{ConsoleStream, ConsoleStreamType, Reporter};
use starbase_styles::color::owo::{OwoColorize, XtermColors};
use starbase_styles::color::{Color, OwoStyle, no_color};
use std::time::Duration;

#[derive(Debug, Default)]
pub struct PipelineReportItem {
    pub duration: Option<Duration>,
    pub summarize: bool,
    pub status: ActionPipelineStatus,
}

#[derive(Clone, Debug, Default)]
pub struct TaskReportItem {
    pub attempt_current: u8,
    pub attempt_total: u8,
    pub hash: Option<String>,
    pub output_prefix: Option<String>,
    pub output_streamed: bool,
    pub output_style: Option<TaskOutputStyle>,
}

const STEP_CHAR: &str = "▪";
const CACHED_COLORS: [u8; 4] = [57, 63, 69, 75]; // blue
const PASSED_COLORS: [u8; 4] = [35, 42, 49, 86]; // green
const FAILED_COLORS: [u8; 4] = [124, 125, 126, 127]; // red
const MUTED_COLORS: [u8; 4] = [240, 242, 244, 246]; // gray
const SETUP_COLORS: [u8; 4] = [198, 205, 212, 219]; // pink
const ANNOUNCEMENT_COLORS: [u8; 4] = [208, 214, 220, 226]; // yellow

#[derive(Clone, Copy)]
pub enum Checkpoint {
    Announcement,
    RunCached,
    RunFailed,
    RunPassed,
    RunStarted,
    Setup,
}

fn bold(message: &str) -> String {
    if no_color() {
        message.to_owned()
    } else {
        OwoStyle::new().style(message).bold().to_string()
    }
}

#[derive(Debug)]
pub struct MoonReporter {
    err: ConsoleStream,
    out: ConsoleStream,
    test_mode: bool,
}

impl MoonReporter {
    pub fn new_testing() -> Self {
        Self {
            err: ConsoleStream::new_testing(ConsoleStreamType::Stderr),
            out: ConsoleStream::new_testing(ConsoleStreamType::Stdout),
            test_mode: true,
        }
    }
}

impl Default for MoonReporter {
    fn default() -> Self {
        Self {
            err: ConsoleStream::empty(ConsoleStreamType::Stderr),
            out: ConsoleStream::empty(ConsoleStreamType::Stdout),
            test_mode: false,
        }
    }
}

impl Reporter for MoonReporter {
    fn inherit_streams(&mut self, err: ConsoleStream, out: ConsoleStream) {
        if !self.test_mode {
            self.err = err;
            self.out = out;
        }
    }
}

impl MoonReporter {
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

    pub fn format_checkpoint<M: AsRef<str>, C: AsRef<[String]>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
        comments: C,
    ) -> String {
        let colors = match checkpoint {
            Checkpoint::Announcement => ANNOUNCEMENT_COLORS,
            Checkpoint::RunCached => CACHED_COLORS,
            Checkpoint::RunFailed => FAILED_COLORS,
            Checkpoint::RunPassed => PASSED_COLORS,
            Checkpoint::RunStarted => MUTED_COLORS,
            Checkpoint::Setup => SETUP_COLORS,
        };

        let mut out = format!(
            "{}{}{}{} {}",
            color::paint(colors[0], STEP_CHAR),
            color::paint(colors[1], STEP_CHAR),
            color::paint(colors[2], STEP_CHAR),
            color::paint(colors[3], STEP_CHAR),
            bold(message.as_ref()),
        );

        let suffix = self.format_comments(comments);

        if !suffix.is_empty() {
            out.push(' ');
            out.push_str(&suffix);
        }

        out
    }

    pub fn format_comments<C: AsRef<[String]>>(&self, comments: C) -> String {
        let comments = comments.as_ref();

        if comments.is_empty() {
            return String::new();
        }

        color::muted(format!("({})", comments.join(", ")))
    }

    pub fn format_entry_key<K: AsRef<str>>(&self, key: K) -> String {
        color::muted_light(format!("{}:", key.as_ref()))
    }

    pub fn print_checkpoint<M: AsRef<str>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
    ) -> miette::Result<()> {
        self.print_checkpoint_with_comments(checkpoint, message, &[])
    }

    pub fn print_checkpoint_with_comments<M: AsRef<str>, C: AsRef<[String]>>(
        &self,
        checkpoint: Checkpoint,
        message: M,
        comments: C,
    ) -> miette::Result<()> {
        if !self.out.is_quiet() {
            self.out
                .write_line(self.format_checkpoint(checkpoint, message, comments))?;
        }

        Ok(())
    }

    pub fn print_entry<K: AsRef<str>, V: AsRef<str>>(
        &self,
        key: K,
        value: V,
    ) -> miette::Result<()> {
        self.out
            .write_line(format!("{} {}", self.format_entry_key(key), value.as_ref()))?;

        Ok(())
    }

    pub fn print_header<M: AsRef<str>>(&self, message: M) -> miette::Result<()> {
        let header = format!(" {} ", message.as_ref().to_uppercase());

        self.out.write_newline()?;
        self.out.write_line(if no_color() {
            header
        } else {
            OwoStyle::new()
                .style(header)
                .bold()
                .color(XtermColors::from(Color::Black as u8))
                .on_color(XtermColors::from(Color::Purple as u8))
                .to_string()
        })?;
        self.out.write_newline()?;

        Ok(())
    }

    pub fn print_task_checkpoint(
        &self,
        target: &Target,
        operation: &Operation,
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        let mut comments = vec![];

        if operation.meta.is_no_operation() {
            comments.push("no op".into());
        } else {
            let status_comment = self.get_status_meta_comment(operation.status, || {
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

            if let Some(duration) = operation.duration {
                if let Some(elapsed) = time::elapsed_opt(duration) {
                    comments.push(elapsed);
                }
            }
        }

        // Do not include the hash while testing, as the hash
        // constantly changes and breaks our local snapshots
        if !is_test_env() {
            if let Some(hash) = &item.hash {
                comments.push(self.get_short_hash(hash));
            }
        }

        self.print_checkpoint_with_comments(
            if operation.has_failed() {
                Checkpoint::RunFailed
            } else if operation.is_cached() {
                Checkpoint::RunCached
            } else if operation.has_passed() {
                Checkpoint::RunPassed
            } else {
                Checkpoint::RunStarted
            },
            target,
            comments,
        )?;

        Ok(())
    }

    pub fn print_operation_output(
        &self,
        operation: &Operation,
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        let print = || -> miette::Result<()> {
            if let Some(output) = operation.get_exec_output() {
                if let Some(out) = &output.stdout {
                    if !out.is_empty() {
                        if let Some(prefix) = &item.output_prefix {
                            self.out.write_line_with_prefix(out.trim(), prefix)?;
                        } else {
                            self.out.write_line(out.trim())?;
                        }
                    }
                }

                if let Some(err) = &output.stderr {
                    if !err.is_empty() {
                        if let Some(prefix) = &item.output_prefix {
                            self.err.write_line_with_prefix(err.trim(), prefix)?;
                        } else {
                            self.err.write_line(err.trim())?;
                        }
                    }
                }
            }

            Ok(())
        };

        match item.output_style {
            // Only show output on failure
            Some(TaskOutputStyle::BufferOnlyFailure) => {
                if operation.has_failed() {
                    print()?;
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
                print()?;
            }
        };

        Ok(())
    }

    fn print_pipeline_failures(&self, actions: &[Action]) -> miette::Result<()> {
        for action in actions {
            if !action.has_failed() {
                continue;
            }

            self.print_checkpoint(
                Checkpoint::RunFailed,
                match &*action.node {
                    ActionNode::RunTask(inner) => inner.target.as_str(),
                    _ => &action.label,
                },
            )?;

            if let Some(attempt) = action.operations.get_last_execution() {
                if attempt.has_failed() {
                    self.print_operation_output(attempt, &TaskReportItem::default())?;
                }
            }

            // Force flush so the output is rendered in the correct order
            self.out.flush()?;
            self.err.flush()?;

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
                ActionStatus::Aborted | ActionStatus::Failed | ActionStatus::TimedOut => {
                    failed_count += 1;
                }
                ActionStatus::Invalid => {
                    invalid_count += 1;
                }
                ActionStatus::Skipped => {
                    skipped_count += 1;
                }
                _ => {}
            };
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

        let counts_message = if counts_message.is_empty() {
            color::muted("0 tasks ran")
        } else {
            counts_message.join(&color::muted(", "))
        };
        let mut elapsed_time = time::elapsed(item.duration.unwrap_or_default());

        if matches!(
            item.status,
            ActionPipelineStatus::Interrupted | ActionPipelineStatus::Terminated
        ) {
            elapsed_time = format!(
                "{} {}",
                elapsed_time,
                color::muted_light(format!("({:?})", item.status).to_lowercase())
            );
        } else if passed_count == cached_count && failed_count == 0 {
            elapsed_time = format!("{} {}", elapsed_time, label_to_the_moon());
        }

        if item.summarize {
            self.print_entry("Actions", counts_message)?;
            self.print_entry("   Time", elapsed_time)?;
        } else {
            self.print_entry("Tasks", counts_message)?;
            self.print_entry(" Time", elapsed_time)?;
        }

        Ok(())
    }

    fn print_pipeline_summary(&self, actions: &[Action]) -> miette::Result<()> {
        for action in actions {
            let status = match action.status {
                ActionStatus::Passed => color::success("pass"),
                ActionStatus::Cached | ActionStatus::CachedFromRemote => color::label("pass"),
                ActionStatus::Aborted | ActionStatus::Failed | ActionStatus::TimedOut => {
                    color::failure("fail")
                }
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

            if let Some(hash) = action.operations.get_hash() {
                comments.push(self.get_short_hash(hash));
            }

            self.out.write_line(format!(
                "{} {} {}",
                status,
                action.label,
                self.format_comments(comments),
            ))?;
        }

        Ok(())
    }
}

// EVENTS

impl MoonReporter {
    pub fn on_action_started(&self, _action: &Action) -> miette::Result<()> {
        Ok(())
    }

    pub fn on_action_completed(
        &self,
        _action: &Action,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        Ok(())
    }

    pub fn on_pipeline_started(&self, _nodes: &[&ActionNode]) -> miette::Result<()> {
        Ok(())
    }

    pub fn on_pipeline_completed(
        &self,
        actions: &[Action],
        item: &PipelineReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        if actions.is_empty() || self.out.is_quiet() {
            return Ok(());
        }

        // A task failed, so instead of showing the stats,
        // we'll render the error that was bubbled up
        if matches!(item.status, ActionPipelineStatus::Aborted) {
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
            self.print_header("Review")?;
            self.print_pipeline_failures(actions)?;
        }

        self.print_header("Summary")?;
        self.print_pipeline_summary(actions)?;

        self.print_header("Stats")?;
        self.print_pipeline_stats(actions, item)?;

        self.out.write_newline()?;

        Ok(())
    }

    // Print a checkpoint when a task execution starts, for each attempt
    pub fn on_task_started(
        &self,
        target: &Target,
        attempt: &Operation,
        item: &TaskReportItem,
    ) -> miette::Result<()> {
        self.print_task_checkpoint(target, attempt, item)?;

        Ok(())
    }

    // If the task has been running for a long time, print a checkpoint
    pub fn on_task_running(&self, target: &Target, secs: u32) -> miette::Result<()> {
        self.print_checkpoint_with_comments(
            Checkpoint::RunStarted,
            target,
            [format!("running for {secs}s")],
        )?;

        Ok(())
    }

    // When an attempt has finished, print the output if captured
    pub fn on_task_finished(
        &self,
        _target: &Target,
        attempt: &Operation,
        item: &TaskReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        // Task output was captured, so there was no output
        // sent to the console, so manually print the logs we have!
        if !item.output_streamed && attempt.has_output() {
            self.print_operation_output(attempt, item)?;
        }

        Ok(())
    }

    // When all attempts have completed, print the final checkpoint
    pub fn on_task_completed(
        &self,
        target: &Target,
        operations: &OperationList,
        item: &TaskReportItem,
        _error: Option<&miette::Report>,
    ) -> miette::Result<()> {
        if let Some(operation) = operations.get_last_process() {
            // If cached, the finished event above is not fired,
            // so handle printing the captured logs here!
            if operation.is_cached() && operation.has_output() {
                self.print_operation_output(operation, item)?;
            }

            // Then print the success checkpoint. The success
            // checkpoint should always appear after the output,
            // and "contain" it within the start checkpoint!
            self.print_task_checkpoint(target, operation, item)?;
        } else if let Some(operation) = operations.last() {
            self.print_task_checkpoint(target, operation, item)?;
        }

        Ok(())
    }
}

fn label_to_the_moon() -> String {
    [
        color::paint(55, "❯"),
        color::paint(56, "❯❯"),
        color::paint(57, "❯ t"),
        color::paint(63, "o t"),
        color::paint(69, "he "),
        color::paint(75, "mo"),
        color::paint(81, "on"),
    ]
    .into_iter()
    .collect::<Vec<_>>()
    .join("")
}
