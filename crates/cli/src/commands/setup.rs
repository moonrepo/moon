use moon_workspace::Workspace;

pub async fn setup(workspace: Workspace) -> Result<(), Box<dyn std::error::Error>> {
    workspace.toolchain.setup().await?;

    Ok(())
}
