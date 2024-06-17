use moon_action::Action;
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct JobContext {
    /// Force aborts running sibling jobs
    pub abort_token: CancellationToken,

    /// Receives cancel/shutdown signals
    pub cancel_token: CancellationToken,

    /// The project graph, for use within actions
    pub project_graph: Arc<ProjectGraph>,

    /// Sends results to the parent pipeline
    pub result_sender: Sender<Action>,

    /// Acquires a permit for concurrency
    pub semaphore: Arc<Semaphore>,
}

impl JobContext {
    pub fn abort(&self) {
        self.abort_token.cancel();
    }

    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_aborted_or_cancelled(&self) -> bool {
        self.abort_token.is_cancelled() || self.cancel_token.is_cancelled()
    }
}
