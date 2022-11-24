use crate::helpers::{create_progress_bar, load_workspace};
use moon_runner::Runner;

pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Downloading and installing tools...");
    let workspace = load_workspace().await?;

    Runner::setup_toolchain(workspace).await?;

    done("Setup complete", true);

    Ok(())
}
