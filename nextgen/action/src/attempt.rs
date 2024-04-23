use crate::action::ActionStatus;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

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

    pub fn finish(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn has_failed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Failed | ActionStatus::FailedAndAbort
        )
    }
}
