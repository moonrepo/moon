use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use moon_logger::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn setup_toolchain(workspace: Arc<RwLock<Workspace>>) -> Result<(), WorkspaceError> {
    debug!(
        target: "moon:task-runner:setup-toolchain",
        "Setting up toolchain",
    );

    let mut workspace_writable = workspace.write().await;
    let workspace = workspace.read().await;

    workspace
        .toolchain
        .setup(&mut workspace_writable.package_json)
        .await?;

    Ok(())
}
