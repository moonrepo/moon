use crate::context::Context;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use std::future::Future;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

#[derive(Debug)]
pub enum JobState {
    Aborted,
    Cancelled,
    Failed,
    Passed,
    Pending,
    Running,
    TimedOut,
}

pub struct JobResult {
    pub finished_at: DateTime<Utc>,
    pub duration: Duration,
    pub error: Option<miette::Report>,
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub state: JobState,
}

pub struct Job<T: Send> {
    pub id: String,
    pub state: JobState,

    func: BoxFuture<'static, miette::Result<T>>,
}

impl<T: 'static + Send> Job<T> {
    pub fn new<F>(id: String, func: F) -> Self
    where
        F: Future<Output = miette::Result<T>> + Send + 'static,
    {
        Self {
            func: Box::pin(func),
            id,
            state: JobState::Pending,
        }
    }

    pub async fn run(self, context: Context<T>) -> miette::Result<()> {
        let id = self.id;
        let func = self.func;

        debug!(job = &id, "Running job");

        let started_at = Utc::now();
        let duration = Instant::now();

        let mut result = JobResult {
            duration: Duration::new(0, 0),
            error: None,
            finished_at: started_at.clone(),
            id: id.clone(),
            started_at,
            state: JobState::Running,
        };

        // TODO emit event

        tokio::select! {
            // Cancel if we receive a shutdown signal
            _ = context.cancel_token.cancelled() => {
                trace!(id = &id, "Cancelling job");

                result.state = JobState::Cancelled;
            }

            // Or run the job to completion
            res = func => match res {
                Ok(res) => {
                    result.state = JobState::Passed;
                },
                Err(err) => {
                    result.error = Some(err);
                    result.state = JobState::Failed;
                },
            },
        };

        debug!(job = &id, state = ?result.state, "Ran job");

        result.finished_at = Utc::now();
        result.duration = duration.elapsed();

        // Send the result or cancel pipeline on failure
        // if context.result_sender.send(result).await.is_err() {
        //     context.cancel_token.cancel();
        // }

        // TODO emit event

        Ok(())
    }
}
