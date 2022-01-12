use crate::errors::WorkspaceError;
use crate::workspace::Workspace;

#[allow(dead_code)]
pub async fn install_toolchain(workspace: &Workspace) -> Result<(), WorkspaceError> {
    workspace.toolchain.setup().await?;

    Ok(())
}
