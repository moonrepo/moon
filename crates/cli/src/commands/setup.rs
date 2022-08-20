use crate::helpers::{create_progress_bar, load_workspace};

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");

    load_workspace().await?.toolchain.setup(true).await?;

    done("Setup complete");

    Ok(())
}
