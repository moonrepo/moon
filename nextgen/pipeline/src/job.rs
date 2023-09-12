use crate::context::*;
use crate::pipeline_events::*;
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult<T> {
    pub action: Option<T>,
    pub finished_at: DateTime<Utc>,
    pub duration: Duration,
    pub error: Option<String>,
    #[serde(skip)]
    pub error_report: Option<miette::Report>,
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub state: RunState,
}

pub struct Job<T: Send> {
    pub id: String,
    pub state: RunState,

    /// Maximum seconds to run before it's cancelled.
    pub timeout: Option<u64>,

    /// Seconds to emit progress events on an interval.
    pub interval: Option<u64>,

    func: Option<BoxFuture<'static, miette::Result<T>>>,
}

impl<T: 'static + Send> Job<T> {
    pub fn new<F>(id: String, func: F) -> Self
    where
        F: Future<Output = miette::Result<T>> + Send + 'static,
    {
        Self {
            func: Some(Box::pin(func)),
            id,
            state: RunState::Pending,
            timeout: None,
            interval: Some(30),
        }
    }

    pub async fn run(&mut self, context: Context<T>) -> miette::Result<RunState> {
        let func = self.func.take().expect("Missing job action!");

        debug!(job = &self.id, "Running job");

        let started_at = Utc::now();
        let duration = Instant::now();
        let mut action = None;
        let mut error = None;
        let mut error_report = None;

        self.update_state(&context, RunState::Running).await?;

        let timeout_token = CancellationToken::new();
        let timeout_handle = self.track_timeout(timeout_token.clone());
        let progress_handle = self.track_progress(context.clone());

        let final_state = tokio::select! {
            // Abort if a sibling job has failed
            _ = context.abort_token.cancelled() => {
                trace!(job = &self.id, "Job aborted");

                RunState::Aborted
            }

            // Cancel if we receive a shutdown signal
            _ = context.cancel_token.cancelled() => {
                trace!(job = &self.id, "Job cancelled");

                RunState::Cancelled
            }

            // Cancel if we have timed out
            _ = timeout_token.cancelled() => {
                trace!(job = &self.id, "Job timed out");

                RunState::TimedOut
            }

            // Or run the job to completion
            res = func => match res {
                Ok(res) => {
                    action = Some(res);

                    trace!(job = &self.id, "Job passed");

                    RunState::Passed
                },
                Err(e) => {
                    error = Some(e.to_string());
                    error_report = Some(e);

                    trace!(job = &self.id, error = error.as_ref(), "Job failed");

                    RunState::Failed
                },
            },
        };

        self.update_state(&context, final_state).await?;

        timeout_handle.abort();
        progress_handle.abort();

        let result = JobResult {
            action,
            duration: duration.elapsed(),
            error,
            error_report,
            finished_at: Utc::now(),
            id: self.id.clone(),
            started_at,
            state: self.state,
        };

        debug!(
            job = &self.id,
            state = ?result.state,
            duration = ?result.duration,
            "Ran job",
        );

        // context
        //     .on_job_finished
        //     .emit(JobFinishedEvent {
        //         job: id.clone(),
        //         result: result.clone(),
        //     })
        //     .await?;

        // Send the result or cancel pipeline on failure
        if context.result_sender.send(result).await.is_err() {
            context.cancel_token.cancel();
        }

        Ok(self.state)
    }

    async fn update_state(
        &mut self,
        context: &Context<T>,
        next_state: RunState,
    ) -> miette::Result<()> {
        let prev_state = self.state;
        let state = next_state;

        context
            .on_job_state_change
            .emit(JobStateChangeEvent {
                job: self.id.clone(),
                state,
                prev_state,
            })
            .await?;

        self.state = state;

        Ok(())
    }

    fn track_progress(&self, context: Context<T>) -> JoinHandle<()> {
        let duration = self.interval;
        let id = self.id.clone();

        tokio::spawn(async move {
            if let Some(duration) = duration {
                let mut secs = 0;

                loop {
                    sleep(Duration::from_secs(duration)).await;
                    secs += duration;

                    let _ = context
                        .on_job_progress
                        .emit(JobProgressEvent {
                            job: id.clone(),
                            elapsed: secs as u32,
                        })
                        .await;
                }
            }
        })
    }

    fn track_timeout(&self, timeout_token: CancellationToken) -> JoinHandle<()> {
        let duration = self.timeout;

        tokio::spawn(async move {
            if let Some(duration) = duration {
                if timeout(
                    Duration::from_secs(duration),
                    sleep(Duration::from_secs(86400)), // 1 day
                )
                .await
                .is_err()
                {
                    timeout_token.cancel();
                }
            }
        })
    }
}
