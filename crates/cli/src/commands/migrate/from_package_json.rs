use moon_workspace::Workspace;

pub async fn from_package_json(id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Workspace::load().await?;

    Ok(())
}
