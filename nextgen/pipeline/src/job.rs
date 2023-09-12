use crate::context::Context;
use crate::pipeline_events::JobStateChangeEvent;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use std::future::Future;
use std::time::{Duration, Instant};
use tracing::{debug, trace};

#[derive(Clone, Copy, Debug)]
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
    pub timeout: u64,

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
            timeout: 7200, // 2 hours
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

        context
            .on_job_state_change
            .emit(JobStateChangeEvent {
                job: id.clone(),
                state: result.state,
            })
            .await?;

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

        context
            .on_job_state_change
            .emit(JobStateChangeEvent {
                job: id.clone(),
                state: result.state,
            })
            .await?;

        result.finished_at = Utc::now();
        result.duration = duration.elapsed();

        debug!(job = &id, state = ?result.state, duration = ?result.duration, "Ran job");

        // context
        //     .on_job_finished
        //     .emit(JobFinishedEvent {
        //         job: id.clone(),
        //         result: result.clone(),
        //     })
        //     .await?;

        // Send the result or cancel pipeline on failure
        // if context.result_sender.send(result).await.is_err() {
        //     context.cancel_token.cancel();
        // }

        Ok(())
    }
}
