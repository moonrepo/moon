use crate::helpers::{create_progress_bar, AnyError};
use moon::load_workspace;

pub async fn teardown() -> Result<(), AnyError> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    let mut workspace = load_workspace().await?;

    for platform in workspace.platforms.list_mut() {
        platform.teardown_toolchain().await?;
    }

    done("Teardown complete", true);

    Ok(())
}
