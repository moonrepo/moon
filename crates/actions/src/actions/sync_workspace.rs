use crate::operations::{sync_codeowners, sync_vcs_hooks};
use crate::utils::should_skip_action;
use miette::IntoDiagnostic;
use moon_action::{Action, ActionStatus, Operation};
use moon_action_context::ActionContext;
use moon_app_context::AppContext;
use moon_common::{color, is_docker_container};
use moon_project_graph::ProjectGraph;
use std::sync::Arc;
use tokio::task;
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

    // Run operations in parallel
    let mut futures = vec![];

    if app_context.workspace_config.codeowners.sync_on_run {
        debug!(
            "Syncing code owners ({} enabled)",
            color::property("codeowners.syncOnRun"),
        );

        let app_context = Arc::clone(&app_context);
        let project_graph = Arc::clone(&project_graph);

        futures.push(task::spawn(async move {
            Operation::sync_operation("Codeowners")
                .track_async_with_check(
                    || sync_codeowners(&app_context, &project_graph, false),
                    |result| result.is_some(),
                )
                .await
        }));
    }

    if app_context.workspace_config.vcs.sync_hooks {
        debug!(
            "Syncing {} hooks ({} enabled)",
            app_context.workspace_config.vcs.manager,
            color::property("vcs.syncHooks"),
        );

        let app_context = Arc::clone(&app_context);

        futures.push(task::spawn(async move {
            Operation::sync_operation("VCS hooks")
                .track_async_with_check(
                    || sync_vcs_hooks(&app_context, false),
                    |result| result == true,
                )
                .await
        }));
    }

    for future in futures {
        action.operations.push(future.await.into_diagnostic()??);
    }

    Ok(ActionStatus::Passed)
}
