use petgraph::graph::NodeIndex;
use std::time::{Duration, Instant};

pub enum TaskResultStatus {
    Cancelled,
    Failed,
    Invalid,
    Passed,
    Running,
}

pub struct TaskResult {
    pub duration: Option<Duration>,

    pub exit_code: i8,

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
            exit_code: -1,
            node_index,
            start_time: Instant::now(),
            status: TaskResultStatus::Running,
            stderr: String::new(),
            stdout: String::new(),
        }
    }

    pub fn pass(&mut self) {
        self.status = TaskResultStatus::Passed;
        self.duration = Some(self.start_time.elapsed());
    }

    pub fn fail(&mut self) {
        self.status = TaskResultStatus::Failed;
        self.duration = Some(self.start_time.elapsed());
    }
}
