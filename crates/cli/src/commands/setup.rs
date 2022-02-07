use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;
    let mut root_package = workspace.load_package_json()?;

    workspace.toolchain.setup(&mut root_package).await?;

    Ok(())
}
