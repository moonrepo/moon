use crate::components::create_progress_loader;
use crate::session::MoonSession;
use crate::systems::analyze;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn setup(session: MoonSession) -> AppResult {
    let progress = create_progress_loader(
        session.get_console()?,
        "Downloading and installing toolchains...",
    );

    let toolchain_registry = session.get_toolchain_registry().await?;

    analyze::load_toolchain(&toolchain_registry, &session.toolchain_config).await?;

    progress.stop().await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: "Toolchains have been setup!")
            }
        }
    })?;

    Ok(None)
}
