use crate::event_emitter::EventEmitter;
use moon_action::Action;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore, mpsc::Sender};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct JobContext {
    /// Force aborts running jobs
    pub abort_token: CancellationToken,

    /// Receives cancel/shutdown signals
    pub cancel_token: CancellationToken,

    /// Completed jobs (used by the dispatcher)
    pub completed_jobs: Arc<RwLock<FxHashSet<NodeIndex>>>,

    /// Internal pipeline event emitter
    pub emitter: Arc<EventEmitter>,

    /// Sends results to the parent pipeline
    pub result_sender: Sender<Action>,

    /// Currently running jobs (used by the dispatcher)
    pub running_jobs: Arc<RwLock<FxHashMap<NodeIndex, u64>>>,

    /// Acquires a permit for concurrency
    pub semaphore: Arc<Semaphore>,

    /// The registry of all toolchain plugins
    pub toolchain_registry: Arc<ToolchainRegistry>,

    /// The project and task graphs, for use within actions
    pub workspace_graph: Arc<WorkspaceGraph>,
}

impl JobContext {
    pub fn is_aborted_or_cancelled(&self) -> bool {
        self.abort_token.is_cancelled() || self.cancel_token.is_cancelled()
    }

    pub async fn mark_completed(&self, index: NodeIndex) {
        self.running_jobs.write().await.remove(&index);
        self.completed_jobs.write().await.insert(index);
    }

    pub async fn send_result(&self, action: Action) {
        self.mark_completed(NodeIndex::new(action.node_index)).await;

        let _ = self.result_sender.send(action).await;
    }
}
