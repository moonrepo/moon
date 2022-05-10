use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = Workspace::load().await?;

    workspace.toolchain.setup(&workspace.config, true).await?;

    Ok(())
}
