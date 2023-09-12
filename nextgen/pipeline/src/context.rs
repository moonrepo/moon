use crate::pipeline_events::*;
use starbase_events::Emitter;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio_util::sync::CancellationToken;

pub struct Context<T> {
    /// Receives cancel/shutdown signals.
    pub cancel_token: CancellationToken,

    /// Sends results to the parent pipeline.
    pub result_sender: Sender<Option<T>>,

    /// Acquires a permit for concurrency.
    pub semaphore: Arc<Semaphore>,

    pub on_job_finished: Arc<Emitter<JobFinishedEvent>>,
    pub on_job_state_change: Arc<Emitter<JobStateChangeEvent>>,
}

unsafe impl<T> Send for Context<T> {}
unsafe impl<T> Sync for Context<T> {}

impl<T> Context<T> {
    // Don't use native `Clone` since it'll require `T` to be cloneable.
    pub fn clone(&self) -> Context<T> {
        Self {
            cancel_token: self.cancel_token.clone(),
            result_sender: self.result_sender.clone(),
            semaphore: self.semaphore.clone(),
            on_job_finished: self.on_job_finished.clone(),
            on_job_state_change: self.on_job_state_change.clone(),
        }
    }
}
