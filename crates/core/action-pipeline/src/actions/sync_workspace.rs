use super::should_skip_action;
use moon_action::{Action, ActionStatus, Operation, OperationMeta, OperationMetaLabel};
use moon_action_context::ActionContext;
use moon_actions::{sync_codeowners, sync_vcs_hooks};
use moon_common::is_docker_container;
use moon_logger::debug;
use moon_project_graph::ProjectGraph;
use moon_utils::is_test_env;
use moon_workspace::Workspace;
use starbase_styles::color;
use std::env;
use std::sync::Arc;

const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_workspace(
    action: &mut Action,
    _context: Arc<ActionContext>,
    workspace: Arc<Workspace>,
    project_graph: Arc<ProjectGraph>,
) -> miette::Result<ActionStatus> {
    // This causes a lot of churn in tests, revisit
    if !is_test_env() {
        env::set_var("MOON_RUNNING_ACTION", "sync-workspace");
    }

    debug!(target: LOG_TARGET, "Syncing workspace");

    if should_skip_action("MOON_SKIP_SYNC_WORKSPACE") {
        debug!(
            target: LOG_TARGET,
            "Skipping sync workspace action because MOON_SKIP_SYNC_WORKSPACE is set",
        );

        return Ok(ActionStatus::Skipped);
    }

    // Avoid the following features when in Docker
    if is_docker_container() {
        return Ok(ActionStatus::Passed);
    }

    if workspace.config.codeowners.sync_on_run {
        let mut operation =
            Operation::new(OperationMeta::SyncOperation(Box::new(OperationMetaLabel {
                label: "Codeowners".into(),
            })));

        debug!(
            target: LOG_TARGET,
            "Syncing code owners ({} enabled)",
            color::property("codeowners.syncOnRun"),
        );

        let result = sync_codeowners(&workspace, &project_graph, false).await?;

        operation.finish(if result.is_some() {
            ActionStatus::Passed
        } else {
            ActionStatus::Skipped
        });

        action.operations.push(operation);
    }

    if workspace.config.vcs.sync_hooks {
        let mut operation =
            Operation::new(OperationMeta::SyncOperation(Box::new(OperationMetaLabel {
                label: "VCS hooks".into(),
            })));

        debug!(
            target: LOG_TARGET,
            "Syncing {} hooks ({} enabled)",
            workspace.config.vcs.manager,
            color::property("vcs.syncHooks"),
        );

        let result = sync_vcs_hooks(&workspace, false).await?;

        operation.finish(if result {
            ActionStatus::Passed
        } else {
            ActionStatus::Skipped
        });

        action.operations.push(operation);
    }

    Ok(ActionStatus::Passed)
}
