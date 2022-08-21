use crate::helpers::{create_progress_bar, load_workspace};

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    load_workspace().await?.toolchain.teardown().await?;

    done("Teardown complete", true);

    Ok(())
}
