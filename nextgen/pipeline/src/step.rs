use crate::context::*;
use crate::job::*;
use crate::JobAction;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::debug;

async fn spawn_job<T: 'static + Send>(job: Job<T>, context: Context<T>) -> JoinHandle<RunState> {
    let permit = context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to spawn job!");

    tokio::spawn(async move {
        let result = job.run(context).await;

        drop(permit);

        result.ok().unwrap_or(RunState::Failed)
    })
}

#[async_trait]
pub trait Step<T>: Send {
    fn get_job_count(&self) -> usize;
    async fn run(self: Box<Self>, context: Context<T>) -> RunState;
}

pub struct IsolatedStep<T: Send> {
    job: Job<T>,
}

impl<T: 'static + Send> IsolatedStep<T> {
    pub fn new(id: String, action: impl JobAction<T> + 'static) -> Self {
        Self {
            job: Job::new(id, action),
        }
    }
}

impl<T: 'static + Send> From<Job<T>> for IsolatedStep<T> {
    fn from(job: Job<T>) -> IsolatedStep<T> {
        IsolatedStep { job }
    }
}

#[async_trait]
impl<T: 'static + Send> Step<T> for IsolatedStep<T> {
    fn get_job_count(&self) -> usize {
        1
    }

    async fn run(self: Box<Self>, context: Context<T>) -> RunState {
        let handle = spawn_job(self.job, context).await;

        handle.await.ok().unwrap()
    }
}

pub struct BatchedStep<T: Send> {
    id: Arc<String>,
    jobs: Vec<Job<T>>,
}

impl<T: 'static + Send> BatchedStep<T> {
    pub fn new(id: String) -> Self {
        Self {
            id: Arc::new(id),
            jobs: vec![],
        }
    }

    pub fn add_job(&mut self, mut job: Job<T>) -> &mut Self {
        job.batch_id = Some(self.id.clone());

        self.jobs.push(job);
        self
    }
}

#[async_trait]
impl<T: 'static + Send> Step<T> for BatchedStep<T> {
    fn get_job_count(&self) -> usize {
        self.jobs.len()
    }

    async fn run(self: Box<Self>, context: Context<T>) -> RunState {
        debug!(
            batch = self.id.as_str(),
            job_count = self.jobs.len(),
            "Running batched step"
        );

        let mut batch = Vec::with_capacity(self.jobs.len());
        let mut fail_count = 0;

        for job in self.jobs {
            batch.push(spawn_job(job, context.clone()).await);
        }

        for handle in batch {
            if handle.is_finished() {
                continue;
            }

            // If the pipeline is shutting down, abort the handle
            if context.is_aborted_or_cancelled() {
                handle.abort();
            }

            // If the handle failed from a non-cancellation, abort the pipeline
            if let Err(error) = handle.await {
                fail_count += 1;

                if !error.is_cancelled() || error.is_panic() {
                    context.abort();
                }
            }
        }

        debug!(batch = self.id.as_str(), "Ran batched step");

        if fail_count > 0 {
            RunState::Failed
        } else {
            RunState::Passed
        }
    }
}
