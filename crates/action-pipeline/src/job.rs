use crate::action::run_action;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{instrument, trace};

pub struct Job {
    pub node: ActionNode,
    pub node_index: usize,

    /// Contexts of all the things
    pub context: JobContext,
    pub app_context: Arc<AppContext>,
    pub action_context: Arc<ActionContext>,

    /// Maximum seconds to run before it's cancelled
    pub timeout: Option<u64>,
}

impl Job {
    #[instrument(skip_all)]
    pub async fn dispatch(self) {
        let timeout_token = CancellationToken::new();
        let timeout_handle = self.track_timeout(self.timeout, timeout_token.clone());

        let mut action = Action::new(self.node);
        action.node_index = self.node_index;

        tokio::select! {
            // Run conditions in order!
            biased;

            // Abort if a sibling job has failed
            _ = self.context.abort_token.cancelled() => {
                trace!(
                    index = self.node_index,
                    "Job aborted",
                );

                action.finish(ActionStatus::Aborted);
            }

            // Cancel if we receive a shutdown signal
            _ = self.context.cancel_token.cancelled() => {
                trace!(
                    index = self.node_index,
                    "Job cancelled (via signal)",
                );

                action.finish(ActionStatus::Invalid);
            }

            // Cancel if we have timed out
            _ = timeout_token.cancelled() => {
                trace!(
                    index = self.node_index,
                    "Job timed out",
                );

                action.finish(ActionStatus::TimedOut);
            }

            // Or run the job to completion
            _ = run_action(
                &mut action,
                self.action_context,
                self.app_context,
                self.context.project_graph.clone(),
            ) => {},
        };

        // Cleanup before sending the result
        if let Some(handle) = timeout_handle {
            handle.abort();
        }

        // Send the result back to the pipeline
        self.context.send_result(action).await;
    }

    fn track_timeout(
        &self,
        duration: Option<u64>,
        timeout_token: CancellationToken,
    ) -> Option<JoinHandle<()>> {
        duration.map(|duration| {
            tokio::spawn(async move {
                if timeout(
                    Duration::from_secs(duration),
                    sleep(Duration::from_secs(86400)), // 1 day
                )
                .await
                .is_err()
                {
                    timeout_token.cancel();
                }
            })
        })
    }
}
