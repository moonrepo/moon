use crate::helpers::{create_progress_bar, AnyError};
use moon::load_workspace;

pub async fn teardown() -> Result<(), AnyError> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    load_workspace().await?.toolchain.teardown().await?;

    done("Teardown complete", true);

    Ok(())
}
