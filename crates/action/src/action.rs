use std::time::{Duration, Instant};

pub struct Attempt {
    pub duration: Option<Duration>,

    pub index: u8,

    pub start_time: Instant,
}

impl Attempt {
    pub fn new(index: u8) -> Self {
        Attempt {
            duration: None,
            index,
            start_time: Instant::now(),
        }
    }

    pub fn done(&mut self) {
        self.duration = Some(self.start_time.elapsed());
    }
}

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

pub struct Action {
    pub attempts: Option<Vec<Attempt>>,

    pub duration: Option<Duration>,

    pub error: Option<String>,

    pub label: Option<String>,

    pub node_index: usize,

    pub start_time: Instant,

    pub status: ActionStatus,
}

impl Action {
    pub fn new(node_index: usize, label: Option<String>) -> Self {
        Action {
            attempts: None,
            duration: None,
            error: None,
            label,
            node_index,
            start_time: Instant::now(),
            status: ActionStatus::Running,
        }
    }

    pub fn abort(&mut self) {
        self.status = ActionStatus::FailedAndAbort;
    }

    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.status = ActionStatus::Failed;
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn has_failed(&self) -> bool {
        matches!(self.status, ActionStatus::Failed)
            || matches!(self.status, ActionStatus::FailedAndAbort)
    }

    pub fn pass(&mut self, status: ActionStatus) {
        self.status = status;
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn should_abort(&self) -> bool {
        matches!(self.status, ActionStatus::FailedAndAbort)
    }
}
