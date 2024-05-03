use crate::action::ActionStatus;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::{Deserialize, Serialize};
use std::mem;
use std::process::Output;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AttemptType {
    CacheHydration,
    NoOperation,
    #[default]
    TaskExecution,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attempt {
    pub duration: Option<Duration>,

    pub exit_code: Option<i32>,

    pub finished_at: Option<NaiveDateTime>,

    pub started_at: NaiveDateTime,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,

    pub stderr: Option<Arc<String>>,

    pub stdout: Option<Arc<String>>,

    #[serde(rename = "type")]
    pub type_of: AttemptType,
}

impl Attempt {
    pub fn new(type_of: AttemptType) -> Self {
        Attempt {
            duration: None,
            exit_code: None,
            finished_at: None,
            started_at: now_timestamp(),
            start_time: Some(Instant::now()),
            status: ActionStatus::Running,
            stderr: None,
            stdout: None,
            type_of,
        }
    }

    pub fn finish(&mut self, status: ActionStatus) {
        self.finished_at = Some(now_timestamp());
        self.status = status;

        if let Some(start) = &self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    pub fn finish_from_output(&mut self, output: &mut Output) {
        self.exit_code = output.status.code();

        self.set_stderr(String::from_utf8(mem::take(&mut output.stderr)).unwrap_or_default());

        self.set_stdout(String::from_utf8(mem::take(&mut output.stdout)).unwrap_or_default());

        self.finish(if output.status.success() {
            ActionStatus::Passed
        } else {
            ActionStatus::Failed
        });
    }

    pub fn set_stderr(&mut self, output: String) {
        self.stderr = Some(Arc::new(output));
    }

    pub fn set_stdout(&mut self, output: String) {
        self.stdout = Some(Arc::new(output));
    }

    pub fn has_failed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Failed | ActionStatus::FailedAndAbort
        )
    }

    pub fn has_passed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote | ActionStatus::Passed
        )
    }

    pub fn is_cached(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Cached | ActionStatus::CachedFromRemote
        )
    }
}
