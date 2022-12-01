use crate::helpers::{create_progress_bar, load_workspace, AnyError};

pub async fn teardown() -> Result<(), AnyError> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    load_workspace().await?.toolchain.teardown().await?;

    done("Teardown complete", true);

    Ok(())
}
