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
}

unsafe impl<T> Send for Context<T> {}
unsafe impl<T> Sync for Context<T> {}

impl<T> Context<T> {
    pub fn clone(&self) -> Context<T> {
        Self {
            cancel_token: self.cancel_token.clone(),
            result_sender: self.result_sender.clone(),
            semaphore: self.semaphore.clone(),
        }
    }
}
