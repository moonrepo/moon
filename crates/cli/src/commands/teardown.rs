use moon_workspace::Workspace;

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = Workspace::load().await?;

    workspace.toolchain.teardown().await?;

    Ok(())
}
