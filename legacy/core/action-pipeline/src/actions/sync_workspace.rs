use super::should_skip_action;
use moon_action::{Action, ActionStatus, Operation};
use moon_action_context::ActionContext;
use moon_actions_legacy::{sync_codeowners, sync_vcs_hooks};
use moon_app_context::AppContext;
use moon_common::is_docker_container;
use moon_logger::debug;
use moon_project_graph::ProjectGraph;
use moon_utils::is_test_env;
use starbase_styles::color;
use std::env;
use std::sync::Arc;
use tracing::instrument;

const LOG_TARGET: &str = "moon:action:sync-workspace";

#[instrument(skip_all)]
pub async fn sync_workspace(
    action: &mut Action,
    _context: Arc<ActionContext>,
    app_context: Arc<AppContext>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<ActionStatus> {
    if app_context.workspace_config.codeowners.sync_on_run {
        debug!(
            target: LOG_TARGET,
            "Syncing code owners ({} enabled)",
            color::property("codeowners.syncOnRun"),
        );

        action.operations.push(
            Operation::sync_operation("Codeowners")
                .track_async_with_check(
                    || sync_codeowners(&app_context, &project_graph, false),
                    |result| result.is_some(),
                )
                .await?,
        );
    }

    if app_context.workspace_config.vcs.sync_hooks {
        debug!(
            target: LOG_TARGET,
            "Syncing {} hooks ({} enabled)",
            app_context.workspace_config.vcs.manager,
            color::property("vcs.syncHooks"),
        );

        action.operations.push(
            Operation::sync_operation("VCS hooks")
                .track_async_with_check(|| sync_vcs_hooks(&app_context, false), |result| result)
                .await?,
        );
    }

    Ok(ActionStatus::Passed)
}
