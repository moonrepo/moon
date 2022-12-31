use crate::helpers::{create_progress_bar, AnyError};
use moon::load_workspace_with_toolchain;

pub async fn setup() -> Result<(), AnyError> {
    let done = create_progress_bar("Downloading and installing tools...");

    load_workspace_with_toolchain().await?;

    done("Setup complete", true);

    Ok(())
}
