use crate::event_emitter::EventEmitter;
use moon_action::Action;
use moon_daemon_client::DaemonClient;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc::Sender, mpsc::UnboundedSender};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct JobContext {
    /// Force aborts running jobs
    pub abort_token: CancellationToken,

    /// Abort the pipeline when any job fails, not just hard failures
    pub bail: bool,

    /// Receives cancel/shutdown signals
    pub cancel_token: CancellationToken,

    /// Sends jobs that have completed to the dispatcher, which drains the
    /// queue and tracks the completions itself
    pub completed_queue: UnboundedSender<NodeIndex>,

    /// Optional daemon client for use within actions.
    pub daemon_client: Option<DaemonClient>,

    /// Internal pipeline event emitter
    pub emitter: Arc<EventEmitter>,

    /// Sends results to the parent pipeline
    pub result_sender: Sender<Action>,

    /// Currently running jobs (used by the dispatcher)
    pub running_jobs: Arc<RwLock<FxHashMap<NodeIndex, u64>>>,

    /// Acquires a permit for concurrency
    pub semaphore: Arc<Semaphore>,

    /// The project and task graphs, for use within actions
    pub workspace_graph: Arc<WorkspaceGraph>,
}

impl JobContext {
    pub fn is_aborted_or_cancelled(&self) -> bool {
        self.abort_token.is_cancelled() || self.cancel_token.is_cancelled()
    }

    pub async fn mark_completed(&self, index: NodeIndex) {
        self.running_jobs.write().await.remove(&index);

        // Fails when the dispatcher has stopped receiving (the pipeline
        // was aborted), in which case nothing is waiting on this job
        let _ = self.completed_queue.send(index);
    }

    /// Whether the action should abort the entire pipeline.
    pub fn should_abort(&self, action: &Action) -> bool {
        action.should_abort() || self.bail && action.should_bail()
    }

    pub async fn send_result(&self, action: Action) {
        // Abort *before* marking the job as completed. The dispatcher releases
        // dependents the moment their dependencies are in the completed set,
        // and the pipeline only cancels the token once it receives this result,
        // so a dependent could otherwise be dispatched (and run to completion
        // in a broken environment) in between.
        if self.should_abort(&action) {
            self.abort_token.cancel();
        }

        self.mark_completed(NodeIndex::new(action.node_index)).await;

        let _ = self.result_sender.send(action).await;
    }
}
