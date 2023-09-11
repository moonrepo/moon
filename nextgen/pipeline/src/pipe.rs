use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Semaphore;

#[async_trait::async_trait]
pub trait Pipe: Send + Sync {
    async fn run(&self);
}

pub struct PipeHandle {
    pipe: Box<dyn Pipe>,
    semaphore: Arc<Semaphore>,
    sender: Sender<u8>,
}

impl PipeHandle {
    pub fn new(pipe: Box<dyn Pipe>, semaphore: Arc<Semaphore>, sender: Sender<u8>) -> Self {
        Self {
            pipe,
            semaphore,
            sender,
        }
    }

    pub async fn run(self) {
        let Ok(permit) = self.semaphore.acquire_owned().await else {
            return; // Should error?
        };

        let job = self.pipe;

        let handle = tokio::spawn(async move {
            let result = job.run().await;
            drop(permit);
            result
        });

        handle.await.unwrap();
    }
}
