use crate::deps::install_node_modules;
use moon_action::{Action, ActionContext, ActionStatus};
use moon_config::ProjectID;
use moon_contract::SupportedPlatform;
use moon_workspace::{Workspace, WorkspaceError};
use std::sync::Arc;
use tokio::sync::RwLock;

const LOG_TARGET: &str = "moon:platform-node:install-project-deps";

pub async fn install_project_deps(
    _action: &mut Action,
    context: &ActionContext,
    workspace: Arc<RwLock<Workspace>>,
    platform: &SupportedPlatform,
    project_id: &ProjectID,
) -> Result<ActionStatus, WorkspaceError> {
    let workspace = workspace.read().await;
    let project = workspace.projects.load(&project_id)?;
    let node = workspace.toolchain.node.get_from_platform(&platform)?;
    let pm = node.get_package_manager();

    // Check if the lockfile or manifest has been modified in the project
    let has_modified_files = context
        .touched_files
        .contains(&project.root.join(pm.get_lock_filename()))
        || context
            .touched_files
            .contains(&project.root.join(pm.get_manifest_filename()));

    install_node_modules(
        &workspace,
        &platform,
        &node,
        &project.root,
        Some(&project_id),
        &format!("{}:{}", LOG_TARGET, project_id),
        has_modified_files,
    )
    .await
}
