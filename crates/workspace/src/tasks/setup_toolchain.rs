use crate::errors::WorkspaceError;
use crate::workspace::Workspace;
use async_recursion::async_recursion;
use moon_logger::debug;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_recursion(?Send)]
pub async fn setup_toolchain(workspace: Arc<RwLock<Workspace>>) -> Result<(), WorkspaceError> {
    debug!(
        target: "moon:task-runner:setup-toolchain",
        "Setting up toolchain",
    );

    let workspace = workspace.read().await;
    let mut root_package = workspace.load_package_json().await?;

    workspace.toolchain.setup(&mut root_package).await?;

    Ok(())
}
