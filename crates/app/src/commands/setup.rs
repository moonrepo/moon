use crate::components::create_progress_loader;
use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_pdk_api::SetupToolchainInput;
use moon_platform::PlatformManager;
use starbase::AppResult;
use tracing::instrument;

#[instrument]
pub async fn setup(session: MoonSession) -> AppResult {
    let progress = create_progress_loader(
        session.get_console()?,
        "Downloading and installing toolchains...",
    );

    for platform in PlatformManager::write().list_mut() {
        platform.setup_toolchain().await?;
    }

    session
        .get_toolchain_registry()
        .await?
        .setup_toolchain_all(|registry, toolchain| SetupToolchainInput {
            configured_version: session
                .toolchain_config
                .plugins
                .get(toolchain.id.as_str())
                .and_then(|plugin| plugin.version.clone()),
            context: registry.create_context(),
            toolchain_config: registry.create_config(&toolchain.id, &session.toolchain_config),
            version: None,
        })
        .await?;

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
