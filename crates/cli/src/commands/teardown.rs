use moon_workspace::Workspace;

pub async fn teardown(workspace: Workspace) -> Result<(), Box<dyn std::error::Error>> {
    workspace.toolchain.teardown().await?;

    Ok(())
}
