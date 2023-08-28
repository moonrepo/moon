use crate::helpers::create_progress_bar;
use moon_platform::PlatformManager;
use starbase::system;

#[system]
pub async fn teardown() {
    let done = create_progress_bar("Tearing down toolchain and uninstalling tools...");

    for platform in PlatformManager::write().list_mut() {
        platform.teardown_toolchain().await?;
    }

    done("Teardown complete", true);
}
