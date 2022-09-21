use crate::helpers::load_workspace;

pub async fn prune() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = load_workspace().await?;

    Ok(())
}
