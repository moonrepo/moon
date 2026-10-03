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

    /// Whether the job cleans up after other jobs (a `cleanup` task
    /// dependency), which must run even if the pipeline was aborted
    pub cleanup: bool,

    /// Contexts of all the things
    pub context: JobContext,
    pub app_context: Arc<AppContext>,
    pub action_context: Arc<ActionContext>,
}

impl Job {
    #[instrument(skip_all)]
    pub async fn dispatch(self) {
        let Job {
            node,
            node_index,
            cleanup,
            context,
            app_context,
            action_context,
        } = self;

        let mut action = Action::new(node);
        action.node_index = node_index;

        // Cleanup jobs are never aborted (only cancelled), as they
        // must still clean up after the jobs that have ran
        let is_aborted = || !cleanup && context.abort_token.is_cancelled();
        let aborted_before_start = context.abort_token.is_cancelled();

        // They must wait for running processes to be terminated
        // first though, otherwise their own process would be as well
        if cleanup && aborted_before_start {
            debug!(
                index = node_index,
                "Pipeline was aborted, waiting to run cleanup job"
            );

            context
                .abort_state
                .wait_until_resumable(&context.cancel_token)
                .await;
        }

        // The pipeline may have been aborted (a sibling failed) or cancelled
        // (a signal) while this job was queued or waiting for a permit. Don't
        // start it, as it would run against a broken environment.
        if is_aborted() {
            debug!(index = node_index, "Job aborted before it was started");

            action.finish(ActionStatus::Aborted);
            context.send_result(action).await;

            return;
        } else if context.cancel_token.is_cancelled() {
            debug!(
                index = node_index,
                "Job cancelled before it was started (because a signal)"
            );

            action.finish(ActionStatus::Skipped);
            context.send_result(action).await;

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
            action_context,
            app_context,
            context.clone(),
        ))
        .await
        .is_err()
        {
            action.finish(ActionStatus::Failed);
        };

        // Abort if a sibling job has failed, in which case this job was
        // terminated because of the pipeline, and not because of itself.
        // A cleanup job that was running at the time was terminated as well
        if is_aborted()
            || (cleanup
                && !aborted_before_start
                && context.abort_token.is_cancelled()
                && action.has_failed())
        {
            debug!(index = node_index, "Job aborted");

            context.abort_state.mark_casualty(node_index);
            action.finish(ActionStatus::Aborted);
        }
        // Cancel if we receive a shutdown signal
        else if context.cancel_token.is_cancelled() {
            debug!(index = node_index, "Job cancelled (because a signal)");

            action.finish(ActionStatus::Skipped);
        }

        // Send the result back to the pipeline
        context.send_result(action).await;
    }
}
