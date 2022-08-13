use crate::helpers::create_progress_bar;
use moon_workspace::Workspace;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");

    Workspace::load().await?.toolchain.setup(true).await?;

    done("Setup complete");

    Ok(())
}
