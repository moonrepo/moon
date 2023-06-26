use moon_action::{Action, ActionStatus};
use moon_action_context::ActionContext;
use moon_actions::{sync_codeowners, sync_vcs_hooks};
use moon_logger::debug;
use moon_project_graph::ProjectGraph;
use moon_utils::is_test_env;
use moon_workspace::Workspace;
use starbase_styles::color;
use std::env;
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:action:sync-workspace";

pub async fn sync_workspace(
    _action: &mut Action,
    _context: Arc<RwLock<ActionContext>>,
    workspace: Arc<RwLock<Workspace>>,
    project_graph: Arc<RwLock<ProjectGraph>>,
) -> miette::Result<ActionStatus> {
    // This causes a lot of churn in tests, revisit
    if !is_test_env() {
        env::set_var("MOON_RUNNING_ACTION", "sync-workspace");
    }

    let workspace = workspace.read().await;
    let project_graph = project_graph.read().await;

    debug!(target: LOG_TARGET, "Syncing workspace");

    if workspace.config.codeowners.sync_on_run {
        debug!(
            target: LOG_TARGET,
            "Syncing code owners ({} enabled)",
            color::id("codeowners.syncOnRun"),
        );

        sync_codeowners(&workspace, &project_graph, false).await?;
    }

    if workspace.config.vcs.sync_hooks {
        debug!(
            target: LOG_TARGET,
            "Syncing {} hooks ({} enabled)",
            workspace.config.vcs.manager,
            color::id("vcs.syncHooks"),
        );

        sync_vcs_hooks(&workspace, false).await?;
    }

    Ok(ActionStatus::Passed)
}
