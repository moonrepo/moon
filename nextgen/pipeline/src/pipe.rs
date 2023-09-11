use std::future::Future;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, Semaphore};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::trace;

#[async_trait::async_trait]
pub trait Pipe: Send {
    fn get_id(&self) -> &str;
    async fn run(self: Box<Self>, handle: PipeHandle) -> JoinHandle<()>;
}

#[derive(Clone)]
pub struct PipeHandle {
    /// Receives cancel/shutdown signals.
    pub cancel_token: CancellationToken,

    /// Sends results to the parent pipeline.
    pub result_sender: Sender<u8>,

    /// Acquires a permit for concurrency.
    pub semaphore: Arc<Semaphore>,
}

impl PipeHandle {
    pub async fn run<T>(self, job: T) -> JoinHandle<()>
    where
        T: Future + Send + 'static,
        T::Output: Send + 'static,
    {
        let permit = self.semaphore.acquire_owned().await.unwrap();
        let result_sender = self.result_sender;
        let cancel_token = self.cancel_token;

        tokio::spawn(async move {
            let result = tokio::select! {
                // Cancel if we receive a shutdown signal
                _ = cancel_token.cancelled() => {
                    trace!("Cancelling job");
                    false
                }
                // Or run the job to completion
                _ = job => {
                    trace!("Waiting for job to complete");
                    true
                }
            };

            // Send the result or cancel pipeline on failure
            if result_sender.send(result as u8).await.is_err() {
                cancel_token.cancel();
            }

            drop(permit);
            ()
        })
    }
}
