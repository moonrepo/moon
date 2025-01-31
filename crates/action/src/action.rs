use crate::action_node::ActionNode;
use crate::operation_list::OperationList;
use moon_time::chrono::NaiveDateTime;
use moon_time::now_timestamp;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionPipelineStatus {
    Aborted,
    Completed,
    Interrupted,
    Terminated,
    #[default]
    Pending,
}

#[derive(Copy, Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    Cached,
    CachedFromRemote,
    Failed,
    Invalid,
    Passed,
    #[default]
    Running,
    Skipped, // When nothing happened

    // Pipeline
    TimedOut,
    Aborted,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub allow_failure: bool,

    pub created_at: NaiveDateTime,

    pub duration: Option<Duration>,

    pub error: Option<String>,

    #[serde(skip)]
    pub error_report: Option<miette::Report>,

    pub finished_at: Option<NaiveDateTime>,

    pub flaky: bool,

    pub label: String,

    pub node: Arc<ActionNode>,

    pub node_index: usize,

    pub operations: OperationList,

    pub started_at: Option<NaiveDateTime>,

    #[serde(skip)]
    pub start_time: Option<Instant>,

    pub status: ActionStatus,
}

impl Action {
    pub fn new(node: ActionNode) -> Self {
        Action {
            allow_failure: false,
            created_at: now_timestamp(),
            duration: None,
            error: None,
            error_report: None,
            finished_at: None,
            flaky: false,
            label: node.label(),
            node: Arc::new(node),
            node_index: 0,
            operations: OperationList::default(),
            started_at: None,
            start_time: None,
            status: ActionStatus::Running,
        }
    }

    pub fn abort(&mut self) {
        self.status = ActionStatus::Aborted;
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
    }

    pub fn has_failed(&self) -> bool {
        matches!(
            &self.status,
            ActionStatus::Aborted | ActionStatus::Failed | ActionStatus::TimedOut
        )
    }

    pub fn get_duration(&self) -> &Duration {
        self.duration
            .as_ref()
            .expect("Cannot get action duration, has it finished?")
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

    pub fn should_abort(&self) -> bool {
        matches!(self.status, ActionStatus::Aborted)
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
