use crate::action_runner::run_action;
use crate::job_context::JobContext;
use moon_action::{Action, ActionNode, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use std::sync::Arc;
use tracing::{instrument, trace};

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

                action.finish(ActionStatus::Skipped);
            }

            // Or run the job to completion
            _ = run_action(
                &mut action,
                self.action_context,
                self.app_context,
                self.context.project_graph.clone(),
                self.context.toolchain_registry.clone(),
                self.context.emitter.clone(),
            ) => {},
        };

        // Send the result back to the pipeline
        self.context.send_result(action).await;
    }
}
