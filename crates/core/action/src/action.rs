use moon_action_graph::ActionNode;
use moon_common::color;
use moon_utils::time::{chrono::prelude::*, now_timestamp};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

fn has_failed(status: &ActionStatus) -> bool {
    matches!(status, ActionStatus::Failed) || matches!(status, ActionStatus::FailedAndAbort)
}

#[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    Cached,
    CachedFromRemote,
    Failed,
    FailedAndAbort,
    Invalid,
    Passed,
    #[default]
    Running,
    Skipped, // When nothing happened
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub duration: Option<Duration>,

    pub exit_code: Option<i32>,

    pub finished_at: Option<NaiveDateTime>,

    pub index: u8,

    pub started_at: NaiveDateTime,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,

    pub stderr: Option<String>,

    pub stdout: Option<String>,
}

impl Attempt {
    pub fn new(index: u8) -> Self {
        Attempt {
            duration: None,
            exit_code: None,
            finished_at: None,
            index,
            started_at: now_timestamp(),
            start_time: Some(Instant::now()),
            status: ActionStatus::Running,
            stderr: None,
            stdout: None,
        }
    }

    pub fn done(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn has_failed(&self) -> bool {
        has_failed(&self.status)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub allow_failure: bool,

    pub attempts: Option<Vec<Attempt>>,

    pub created_at: NaiveDateTime,

    pub duration: Option<Duration>,

    pub error: Option<String>,

    #[serde(skip)]
    pub error_report: Option<miette::Report>,

    pub finished_at: Option<NaiveDateTime>,

    pub flaky: bool,

    pub label: String,

    #[serde(skip)]
    pub log_target: String,

    #[serde(skip)]
    pub node: Option<ActionNode>,

    pub started_at: Option<NaiveDateTime>,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Action {
    pub fn new(node: ActionNode) -> Self {
        Action {
            allow_failure: false,
            attempts: None,
            created_at: now_timestamp(),
            duration: None,
            error: None,
            error_report: None,
            finished_at: None,
            flaky: false,
            label: node.label(),
            log_target: String::new(),
            node: Some(node),
            started_at: None,
            start_time: None,
            status: ActionStatus::Running,
        }
    }

    pub fn abort(&mut self) {
        self.status = ActionStatus::FailedAndAbort;
    }

    pub fn start(&mut self) {
        self.started_at = Some(now_timestamp());
        self.start_time = Some(Instant::now());
    }

    pub fn finish(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn fail(&mut self, error: miette::Report) {
        self.error = Some(error.to_string());
        self.error_report = Some(error);
        self.finish(ActionStatus::Failed);
    }

    pub fn has_failed(&self) -> bool {
        has_failed(&self.status)
    }

    pub fn get_error(&mut self) -> miette::Report {
        if let Some(report) = self.error_report.take() {
            return report;
        }

        if let Some(error) = &self.error {
            return miette::miette!("{error}");
        }

        miette::miette!("Unknown error!")
    }

    pub fn set_attempts(&mut self, attempts: Vec<Attempt>, command: &str) -> bool {
        let some_failed = attempts.iter().any(|a| has_failed(&a.status));
        let mut passed = false;

        if let Some(last) = attempts.last() {
            if last.has_failed() {
                let mut message = format!("Failed to run {}", color::shell(command));

                if let Some(code) = last.exit_code {
                    message += " ";
                    message += color::muted_light(format!("(exit code {})", code)).as_str();
                }

                self.error = Some(message);
            } else {
                passed = true;
            }
        } else {
            passed = true;
        }

        self.attempts = Some(attempts);
        self.flaky = some_failed && passed;

        passed
    }

    pub fn should_abort(&self) -> bool {
        matches!(self.status, ActionStatus::FailedAndAbort)
    }

    pub fn should_bail(&self) -> bool {
        !self.allow_failure && self.has_failed()
    }

    pub fn was_cached(&self) -> bool {
        matches!(
            self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote
        )
    }
}
