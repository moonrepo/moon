use crate::helpers::{create_progress_bar, AnyError};
use moon::load_workspace_with_toolchain;

pub async fn teardown() -> Result<(), AnyError> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    // We need to load and setup the toolchain for it to be "available"
    // for it to be torn down... This is super unfortunate.
    // Perhaps there's a better way to implement this command? Is it even required?
    let mut workspace = load_workspace_with_toolchain().await?;

    for platform in workspace.platforms.list_mut() {
        platform.teardown_toolchain().await?;
    }

    done("Teardown complete", true);

    Ok(())
}
