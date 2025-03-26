use crate::components::create_progress_loader;
use crate::session::CliSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_platform::PlatformManager;
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn teardown(session: CliSession) -> AppResult {
    let progress = create_progress_loader(
        session.get_console()?,
        "Tearing down and uninstalling tools...",
    );

    for platform in PlatformManager::write().list_mut() {
        platform.teardown_toolchain().await?;
    }

    progress.stop().await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: "Toolchain has been torn down!")
            }
        }
    })?;

    Ok(None)
}
