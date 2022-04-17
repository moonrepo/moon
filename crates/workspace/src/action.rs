use petgraph::graph::NodeIndex;
use std::time::{Duration, Instant};

pub enum ActionStatus {
    Cached,
    // CachedFromRemote, // TODO
    Failed,
    Invalid,
    Passed,
    Running,
    Skipped, // When nothing happened
}

pub struct Action {
    pub duration: Option<Duration>,

    pub error: Option<String>,

    pub label: Option<String>,

    pub node_index: NodeIndex,

    pub start_time: Instant,

    pub status: ActionStatus,

    pub stderr: String,

    pub stdout: String,
}

impl Action {
    pub fn new(node_index: NodeIndex) -> Self {
        Action {
            duration: None,
            error: None,
            label: None,
            node_index,
            start_time: Instant::now(),
            status: ActionStatus::Running,
            stderr: String::new(),
            stdout: String::new(),
        }
    }

    pub fn pass(&mut self, status: ActionStatus) {
        self.status = status;
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.status = ActionStatus::Failed;
        self.duration = Some(self.start_time.elapsed());
    }
}
