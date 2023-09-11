use crate::pipe::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use std::future::Future;
use tokio::task::JoinHandle;
use tracing::debug;

pub struct Job {
    pub batch_id: Option<String>,
    pub id: String,

    func: BoxFuture<'static, ()>,
}

impl Job {
    pub fn new<T>(id: String, func: T) -> Self
    where
        T: Future<Output = ()> + Send + 'static,
    {
        Self {
            batch_id: None,
            id,
            func: Box::pin(func),
        }
    }
}

#[async_trait]
impl Pipe for Job {
    fn get_id(&self) -> &str {
        &self.id
    }

    async fn run(self: Box<Self>, handle: PipeHandle) -> JoinHandle<()> {
        debug!(
            batch = self.batch_id.as_ref(),
            job = &self.id,
            "Running job"
        );

        handle.run(self.func).await
    }
}
