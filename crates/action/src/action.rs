use moon_utils::time::chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    Cached,
    // CachedFromRemote, // TODO
    Failed,
    FailedAndAbort,
    Invalid,
    Passed,
    Running,
    Skipped, // When nothing happened
}

#[derive(Deserialize, Serialize)]
pub struct Attempt {
    pub created_at: DateTime<Utc>,

    pub duration: Option<Duration>,

    pub finished_at: Option<DateTime<Utc>>,

    pub index: u8,

    pub started_at: Option<DateTime<Utc>>,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Attempt {
    pub fn new(index: u8) -> Self {
        Attempt {
            created_at: Utc::now(),
            duration: None,
            finished_at: None,
            index,
            started_at: None,
            start_time: None,
            status: ActionStatus::Running,
        }
    }

    pub fn start(&mut self) {
        self.started_at = Some(Utc::now());
        self.start_time = Some(Instant::now());
    }

    pub fn stop(&mut self, status: ActionStatus) {
        self.finished_at = Some(Utc::now());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Action {
    pub attempts: Option<Vec<Attempt>>,

    pub created_at: DateTime<Utc>,

    pub duration: Option<Duration>,

    pub error: Option<String>,

    pub label: Option<String>,

    pub node_index: usize,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Action {
    pub fn new(node_index: usize, label: Option<String>) -> Self {
        Action {
            attempts: None,
            created_at: Utc::now(),
            duration: None,
            error: None,
            label,
            node_index,
            start_time: Some(Instant::now()),
            status: ActionStatus::Running,
        }
    }

    pub fn abort(&mut self) {
        self.status = ActionStatus::FailedAndAbort;
    }

    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.stop(ActionStatus::Failed);
    }

    pub fn has_failed(&self) -> bool {
        matches!(self.status, ActionStatus::Failed)
            || matches!(self.status, ActionStatus::FailedAndAbort)
    }

    pub fn set_attempts(&mut self, attempts: Vec<Attempt>) -> bool {
        let passed = attempts
            .iter()
            .all(|a| matches!(a.status, ActionStatus::Passed));

        self.attempts = Some(attempts);

        passed
    }

    pub fn stop(&mut self, status: ActionStatus) {
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn should_abort(&self) -> bool {
        matches!(self.status, ActionStatus::FailedAndAbort)
    }
}
