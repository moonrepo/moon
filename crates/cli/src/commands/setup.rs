use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let mut workspace = Workspace::load()?;

    workspace
        .toolchain
        .setup(&mut workspace.package_json)
        .await?;

    Ok(())
}
