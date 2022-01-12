use crate::errors::WorkspaceError;
use crate::workspace::Workspace;

pub async fn install_node_deps(workspace: &Workspace) -> Result<(), WorkspaceError> {
    let toolchain = &workspace.toolchain;
    let manager = toolchain.get_package_manager();

    manager.install_deps(toolchain).await?;
    manager.dedupe_deps(toolchain).await?;

    Ok(())
}
