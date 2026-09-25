use crate::abort_state::AbortState;
use crate::event_emitter::EventEmitter;
use moon_action::Action;
use moon_daemon_client::DaemonClient;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc::Sender, mpsc::UnboundedSender};
use tokio_util::sync::CancellationToken;

/// A job that has completed, as sent to the dispatcher.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompletedJob {
    pub index: NodeIndex,

    /// Whether the job's action started running, instead of being
    /// aborted or skipped before it could start.
    pub started: bool,

    /// Whether the job's action failed, or was aborted.
    pub failed: bool,

    /// Whether the job's action executed a task's command, instead of the
    /// task being skipped, or hydrated from the cache.
    pub executed: bool,
}

#[derive(Clone)]
pub struct JobContext {
    /// Force aborts running jobs
    pub abort_token: CancellationToken,

    /// State for handling an aborted pipeline
    pub abort_state: Arc<AbortState>,

    /// Abort the pipeline when any job fails, not just hard failures
    pub bail: bool,

    /// Receives cancel/shutdown signals
    pub cancel_token: CancellationToken,

    /// Sends jobs that have completed to the dispatcher, which drains the
    /// queue and tracks the completions itself
    pub completed_queue: UnboundedSender<CompletedJob>,

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

    /// Mark a job as completed without a result, like a persistent job
    /// (which never completes) that has been dispatched.
    pub async fn mark_completed(&self, index: NodeIndex, started: bool) {
        self.mark_completed_job(CompletedJob {
            index,
            started,
            failed: false,
            executed: false,
        })
        .await;
    }

    async fn mark_completed_job(&self, job: CompletedJob) {
        self.running_jobs.write().await.remove(&job.index);

        // Fails when the dispatcher has stopped receiving (the pipeline
        // was aborted), in which case nothing is waiting on this job
        let _ = self.completed_queue.send(job);
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

        self.mark_completed_job(CompletedJob {
            index: NodeIndex::new(action.node_index),
            started: action.started_at.is_some(),
            failed: action.has_failed(),
            executed: action.operations.has_executed_task(),
        })
        .await;

        let _ = self.result_sender.send(action).await;
    }
}
