use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load()?;

    workspace.toolchain.setup().await?;

    Ok(())
}
