use moon_workspace::Workspace;

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;

    workspace.toolchain.teardown().await?;

    Ok(())
}
