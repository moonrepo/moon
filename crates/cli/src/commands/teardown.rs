use crate::helpers::create_progress_bar;
use moon_workspace::Workspace;

pub async fn teardown() -> Result<(), Box<dyn std::error::Error>> {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    Workspace::load().await?.toolchain.teardown().await?;

    done("Teardown complete");

    Ok(())
}
