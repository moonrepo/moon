use crate::context::Context;
use crate::job::*;
use async_trait::async_trait;
use std::future::Future;
use tokio::task::JoinHandle;
use tracing::debug;

async fn spawn_job<T: 'static + Send>(
    context: Context<T>,
    mut job: Job<T>,
) -> JoinHandle<miette::Result<()>> {
    let permit = context
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .expect("Failed to acquire semaphore!");

    tokio::spawn(async move {
        job.run(context).await?;
        drop(permit);
        Ok(())
    })
}

#[async_trait]
pub trait Step<T>: Send {
    async fn run(self: Box<Self>, context: Context<T>) -> JoinHandle<()>;
}

pub struct IsolatedStep<T: Send> {
    job: Job<T>,
}

impl<T: 'static + Send> IsolatedStep<T> {
    pub fn new<F>(id: String, func: F) -> Self
    where
        F: Future<Output = miette::Result<T>> + Send + 'static,
    {
        Self {
            job: Job::new(id, func),
        }
    }
}

#[async_trait]
impl<T: 'static + Send> Step<T> for IsolatedStep<T> {
    async fn run(self: Box<Self>, context: Context<T>) -> JoinHandle<()> {
        // TODO: abort

        tokio::spawn(async {
            spawn_job(context, self.job).await.await.unwrap().unwrap();
        })
    }
}

pub struct BatchedStep<T: Send> {
    id: String,
    jobs: Vec<Job<T>>,
}

impl<T: 'static + Send> BatchedStep<T> {
    pub fn new(id: String) -> Self {
        Self { id, jobs: vec![] }
    }

    pub fn add_job(&mut self, mut job: Job<T>) -> &mut Self {
        job.id = format!("{}::{}", self.id, job.id);

        self.jobs.push(job);
        self
    }
}

#[async_trait]
impl<T: 'static + Send> Step<T> for BatchedStep<T> {
    async fn run(self: Box<Self>, context: Context<T>) -> JoinHandle<()> {
        debug!(step = &self.id, "Running batched step");

        let mut batch = vec![];

        for job in self.jobs {
            batch.push(spawn_job(context.clone(), job).await);
        }

        // TODO: abort

        tokio::spawn(async move {
            for job in batch {
                job.await.unwrap().unwrap();
            }
        })
    }
}
