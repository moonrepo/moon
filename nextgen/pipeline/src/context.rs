use crate::{job::JobResult, pipeline_events::*};
use serde::{Deserialize, Serialize};
use starbase_events::Emitter;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct Context<T> {
    /// Force aborts running sibling jobs.
    pub abort_token: CancellationToken,

    /// Receives cancel/shutdown signals.
    pub cancel_token: CancellationToken,

    /// Sends results to the parent pipeline.
    pub result_sender: Sender<JobResult<T>>,

    /// Acquires a permit for concurrency.
    pub semaphore: Arc<Semaphore>,

    // Events:
    pub on_job_finished: Arc<Emitter<JobFinishedEvent>>,
    pub on_job_progress: Arc<Emitter<JobProgressEvent>>,
    pub on_job_state_change: Arc<Emitter<JobStateChangeEvent>>,
}

unsafe impl<T> Send for Context<T> {}
unsafe impl<T> Sync for Context<T> {}

impl<T> Context<T> {
    // Don't use native `Clone` since it'll require `T` to be cloneable.
    #[allow(clippy::should_implement_trait)]
    pub fn clone(&self) -> Context<T> {
        Self {
            abort_token: self.abort_token.clone(),
            cancel_token: self.cancel_token.clone(),
            result_sender: self.result_sender.clone(),
            semaphore: self.semaphore.clone(),
            on_job_finished: self.on_job_finished.clone(),
            on_job_progress: self.on_job_progress.clone(),
            on_job_state_change: self.on_job_state_change.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunState {
    /// Job was explicitly aborted via the action.
    Aborted,

    /// Cancelled via a signal (ctrl+c, etc).
    Cancelled,

    /// Job failed.
    Failed,

    /// Job passed.
    Passed,

    /// Job is waiting to run.
    Pending,

    /// Job is currently running and executing action.
    Running,

    /// Cancelled via a timeout.
    TimedOut,
}

impl RunState {
    pub fn has_failed(&self) -> bool {
        matches!(
            self,
            Self::Aborted | Self::Cancelled | Self::Failed | Self::TimedOut
        )
    }
}
