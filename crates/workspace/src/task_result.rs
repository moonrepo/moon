use petgraph::graph::NodeIndex;
use std::time::{Duration, Instant};

pub enum TaskResultStatus {
    Cached,
    Failed,
    Invalid,
    Passed,
    Running,
}

pub struct TaskResult {
    pub duration: Option<Duration>,

    pub error: Option<String>,

    pub exit_code: i8,

    pub label: Option<String>,

    pub node_index: NodeIndex,

    pub start_time: Instant,

    pub status: TaskResultStatus,

    pub stderr: String,

    pub stdout: String,
}

impl TaskResult {
    pub fn new(node_index: NodeIndex) -> Self {
        TaskResult {
            duration: None,
            error: None,
            exit_code: -1,
            label: None,
            node_index,
            start_time: Instant::now(),
            status: TaskResultStatus::Running,
            stderr: String::new(),
            stdout: String::new(),
        }
    }

    pub fn pass(&mut self, status: TaskResultStatus) {
        self.status = status;
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn fail(&mut self, error: String) {
        self.error = Some(error);
        self.status = TaskResultStatus::Failed;
        self.duration = Some(self.start_time.elapsed());
    }
}
