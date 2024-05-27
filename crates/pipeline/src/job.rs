use crate::context::*;
use crate::job_action::JobAction;
use crate::pipeline_events::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace, warn};

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
    pub batch_id: Option<Arc<String>>,
    pub id: String,

    /// Maximum seconds to run before it's cancelled.
    pub timeout: Option<u64>,

    /// Seconds to emit progress events on an interval.
    pub interval: Option<u64>,

    action: Box<dyn JobAction<T>>,
}

impl<T: 'static + Send> Job<T> {
    pub fn new(id: String, action: impl JobAction<T> + 'static) -> Self {
        Self {
            action: Box::new(action),
            batch_id: None,
            id,
            timeout: None,
            interval: Some(30),
        }
    }

    pub async fn run(self, context: Context<T>) -> miette::Result<RunState> {
        let action_fn = self.action;
        let batch_id = self.batch_id.as_deref().map(|id| id.as_str());
        let id = self.id;

        debug!(batch = &batch_id, job = &id, "Running job");

        let started_at = Utc::now();
        let duration = Instant::now();
        let mut action = None;
        let mut error = None;
        let mut error_report = None;

        context
            .on_job_state_change
            .emit(JobStateChangeEvent {
                job: id.clone(),
                state: RunState::Running,
                prev_state: RunState::Pending,
            })
            .await?;

        let timeout_token = CancellationToken::new();
        let timeout_handle = track_timeout(self.timeout, timeout_token.clone());
        let progress_handle = track_progress(self.interval, context.clone(), id.clone());

        let final_state = tokio::select! {
            // Run conditions in order!
            biased;

            // Abort if a sibling job has failed
            _ = context.abort_token.cancelled() => {
                trace!(
                    batch = &batch_id,
                    job = &id,
                    "Job aborted",
                );

                RunState::Aborted
            }

            // Cancel if we receive a shutdown signal
            _ = context.cancel_token.cancelled() => {
                trace!(
                    batch = &batch_id,
                    job = &id,
                    "Job cancelled",
                );

                RunState::Cancelled
            }

            // Cancel if we have timed out
            _ = timeout_token.cancelled() => {
                trace!(
                    batch = &batch_id,
                    job = &id,
                    "Job timed out",
                );

                RunState::TimedOut
            }

            // Or run the job to completion
            res = action_fn.run() => match res {
                Ok(res) => {
                    action = Some(res);

                    trace!(
                        batch = &batch_id,
                        job = &id,
                        "Job passed",
                    );

                    RunState::Passed
                },
                Err(e) => {
                    error = Some(e.to_string());
                    error_report = Some(e);

                    trace!(
                        batch = &batch_id,
                        job = &id,
                        error = error.as_ref(),
                        "Job failed",
                    );

                    RunState::Failed
                },
            },
        };

        context
            .on_job_state_change
            .emit(JobStateChangeEvent {
                job: id.clone(),
                state: final_state,
                prev_state: RunState::Running,
            })
            .await?;

        timeout_handle.abort();
        progress_handle.abort();

        let result = JobResult {
            action,
            duration: duration.elapsed(),
            error,
            error_report,
            finished_at: Utc::now(),
            id: id.clone(),
            started_at,
            state: final_state,
        };

        debug!(
            batch = &batch_id,
            job = &id,
            state = ?result.state,
            duration = ?result.duration,
            "Ran job",
        );

        // Send the result to the pipeline
        let _ = context.result_sender.send(result).await;

        Ok(final_state)
    }
}

fn track_progress<T>(duration: Option<u64>, context: Context<T>, id: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        if let Some(duration) = duration {
            let mut secs = 0;

            loop {
                sleep(Duration::from_secs(duration)).await;
                secs += duration;

                if let Err(error) = context
                    .on_job_progress
                    .emit(JobProgressEvent {
                        job: id.clone(),
                        elapsed: secs as u32,
                    })
                    .await
                {
                    warn!(
                        job = &id,
                        error = error.to_string(),
                        "Failed to emit job progress update event!"
                    );
                }
            }
        }
    })
}

fn track_timeout(duration: Option<u64>, timeout_token: CancellationToken) -> JoinHandle<()> {
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
