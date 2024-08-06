use crate::helpers::create_progress_bar;
use moon_platform::PlatformManager;
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn teardown() -> AppResult {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    // TODO
    // for platform in PlatformManager::write().list_mut() {
    //     platform.teardown_toolchain().await?;
    // }

    done("Teardown complete", true);

    Ok(())
}
