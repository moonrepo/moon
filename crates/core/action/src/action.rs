use crate::node::ActionNode;
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

    pub finished_at: Option<NaiveDateTime>,

    pub index: u8,

    pub started_at: NaiveDateTime,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Attempt {
    pub fn new(index: u8) -> Self {
        Attempt {
            duration: None,
            finished_at: None,
            index,
            started_at: now_timestamp(),
            start_time: Some(Instant::now()),
            status: ActionStatus::Running,
        }
    }

    pub fn done(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub attempts: Option<Vec<Attempt>>,

    pub created_at: NaiveDateTime,

    pub duration: Option<Duration>,

    pub error: Option<String>,

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
            attempts: None,
            created_at: now_timestamp(),
            duration: None,
            error: None,
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

    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.finish(ActionStatus::Failed);
    }

    pub fn has_failed(&self) -> bool {
        has_failed(&self.status)
    }

    pub fn set_attempts(&mut self, attempts: Vec<Attempt>) -> bool {
        let some_failed = attempts.iter().any(|a| has_failed(&a.status));
        let passed = match attempts.last() {
            Some(a) => matches!(a.status, ActionStatus::Passed),
            None => true,
        };

        self.attempts = Some(attempts);
        self.flaky = some_failed && passed;

        passed
    }

    pub fn should_abort(&self) -> bool {
        matches!(self.status, ActionStatus::FailedAndAbort)
    }

    pub fn was_cached(&self) -> bool {
        matches!(
            self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote
        )
    }
}
