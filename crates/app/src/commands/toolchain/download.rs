use crate::session::MoonSession;
use clap::Args;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;
use tracing::instrument;

#[derive(Args, Clone, Debug)]
pub struct ToolchainDownloadArgs;

#[instrument(skip(session))]
pub async fn download(session: MoonSession, _args: ToolchainDownloadArgs) -> AppResult {
    let registry = session.get_toolchain_registry().await?;

    if !registry.has_plugin_configs() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Info) {
                    StyledText(content: "No toolchain plugins are configured, unable to download!")
                }
            }
        })?;

        return Ok(None);
    }

    let plugins = registry.load_all().await?;
    let count = plugins.len();

    let message = if count == 1 {
        format!("Downloaded {count} toolchain plugin!")
    } else {
        format!("Downloaded {count} toolchain plugins!")
    };

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(content: message)
            }
        }
    })?;

    Ok(None)
}
