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

    /// Marks the node as complete in the action graph
    pub complete_sender: Option<std::sync::mpsc::Sender<usize>>,

    /// Acquires a permit for concurrency
    pub semaphore: Arc<Semaphore>,
}

impl JobContext {
    pub fn is_aborted_or_cancelled(&self) -> bool {
        self.abort_token.is_cancelled() || self.cancel_token.is_cancelled()
    }

    pub async fn mark_completed(&self, action: Action) {
        if let Some(sender) = &self.complete_sender {
            let _ = sender.send(action.node_index);
        }

        let _ = self.result_sender.send(action).await;
    }
}
