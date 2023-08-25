use crate::helpers::create_progress_bar;
use moon_platform::PlatformManager;
use starbase::AppResult;

pub async fn teardown() -> AppResult {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    // We need to load and setup the toolchain for it to be "available"
    // for it to be torn down... This is super unfortunate.
    // Perhaps there's a better way to implement this command? Is it even required?
    for platform in PlatformManager::write().list_mut() {
        platform.teardown_toolchain().await?;
    }

    done("Teardown complete", true);

    Ok(())
}
