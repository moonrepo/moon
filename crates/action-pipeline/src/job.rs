use crate::action_dispatcher::dispatch;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};

pub struct Job {
    pub node: ActionNode,
    pub index: usize,

    /// Contexts of all the things
    pub context: Arc<JobContext>,
    pub app_context: Arc<AppContext>,
    pub action_context: Arc<ActionContext>,

    /// Maximum seconds to run before it's cancelled
    pub timeout: Option<u64>,
}

impl Job {
    pub async fn dispatch(self) -> ActionStatus {
        let timeout_token = CancellationToken::new();
        let timeout_handle = self.track_timeout(self.timeout, timeout_token.clone());

        let mut action = Action::new(self.node);
        action.node_index = self.index;

        debug!(index = self.index, "Dispatching {} job", action.label);

        tokio::select! {
            // Run conditions in order!
            biased;

            // Abort if a sibling job has failed
            _ = self.context.abort_token.cancelled() => {
                trace!(
                    index = self.index,
                    "Job aborted",
                );

                action.status = ActionStatus::Aborted;
            }

            // Cancel if we receive a shutdown signal
            _ = self.context.cancel_token.cancelled() => {
                trace!(
                    index = self.index,
                    "Job cancelled (via signal)",
                );

                action.status =ActionStatus::Invalid;
            }

            // Cancel if we have timed out
            _ = timeout_token.cancelled() => {
                trace!(
                    index = self.index,
                    "Job timed out",
                );

               action.status = ActionStatus::TimedOut;
            }

            // Or run the job to completion
            result = dispatch(
                &mut action,
                self.action_context,
                self.app_context,
                self.context.project_graph.clone(),
            ) => match result {
                Ok(_) => {
                    trace!(
                        index = self.index,
                        "Job passed",
                    );

                    if matches!(action.status, ActionStatus::Running) {
                        action.status = ActionStatus::Passed;
                    }
                },
                Err(error) => {
                    trace!(
                        index = self.index,
                        "Job failed",
                    );

                    if matches!(action.status, ActionStatus::Running) {
                        action.status = ActionStatus::Failed;
                    }
                },
            },
        };

        debug!(
            index = self.index,
            status = ?action.status,
            "Dispatched {} job", action.label
        );

        // Cleanup before sending the result
        timeout_handle.abort();

        // Send the result back to the pipeline
        let status = action.status;
        let _ = self.context.result_sender.send(action).await;

        status
    }

    fn track_timeout(
        &self,
        duration: Option<u64>,
        timeout_token: CancellationToken,
    ) -> JoinHandle<()> {
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
