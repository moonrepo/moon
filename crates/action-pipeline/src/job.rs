use crate::action_runner::run_action;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use std::sync::Arc;
use tracing::{debug, instrument};

pub struct Job {
    pub node: ActionNode,
    pub node_index: usize,

    /// Contexts of all the things
    pub context: JobContext,
    pub app_context: Arc<AppContext>,
    pub action_context: Arc<ActionContext>,
}

impl Job {
    #[instrument(skip_all)]
    pub async fn dispatch(self) {
        let mut action = Action::new(self.node);
        action.node_index = self.node_index;

        // The pipeline may have been aborted (a sibling failed) or cancelled
        // (a signal) while this job was queued or waiting for a permit. Don't
        // start it, as it would run against a broken environment.
        if self.context.abort_token.is_cancelled() {
            debug!(index = self.node_index, "Job aborted before it was started");

            action.finish(ActionStatus::Aborted);
            self.context.send_result(action).await;

            return;
        } else if self.context.cancel_token.is_cancelled() {
            debug!(
                index = self.node_index,
                "Job cancelled before it was started (because a signal)"
            );

            action.finish(ActionStatus::Skipped);
            self.context.send_result(action).await;

            return;
        }

        // Don't use `tokio::select!` here because if the abort or cancel tokens
        // are triggered, then the async task running the task child process
        // is cancelled, immediately terminating the process, and ignoring
        // any signals we attempt to pass down!

        // Box the future to avoid bloating the (spawned) job future with the
        // entire action state machine, which otherwise overflows the type
        // layout recursion limit. See `run_action` for the nested branches.
        if Box::pin(run_action(
            &mut action,
            self.action_context,
            self.app_context,
            self.context.clone(),
        ))
        .await
        .is_err()
        {
            action.finish(ActionStatus::Failed);
        };

        // Abort if a sibling job has failed
        if self.context.abort_token.is_cancelled() {
            debug!(index = self.node_index, "Job aborted");

            action.finish(ActionStatus::Aborted);
        }
        // Cancel if we receive a shutdown signal
        else if self.context.cancel_token.is_cancelled() {
            debug!(index = self.node_index, "Job cancelled (because a signal)");

            action.finish(ActionStatus::Skipped);
        }

        // Send the result back to the pipeline
        self.context.send_result(action).await;
    }
}
