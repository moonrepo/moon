use crate::utils::*;
use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_docker_container};
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tracing::{debug, instrument};

#[instrument(skip_all)]
pub async fn sync_workspace(
    action: &mut Action,
    _action_context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<ActionStatus> {
    if should_skip_action("MOON_SKIP_SYNC_WORKSPACE") {
        debug!(
            "Skipping workspace sync because {} is set",
            color::symbol("MOON_SKIP_SYNC_WORKSPACE")
        );

        return Ok(ActionStatus::Skipped);
    }

    if is_docker_container() {
        debug!("Skipping workspace sync because we're in a Docker container or image");

        return Ok(ActionStatus::Skipped);
    }

    debug!("Syncing workspace");

    Ok(ActionStatus::Passed)
}
