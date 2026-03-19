use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;

pub async fn start(session: MoonSession) -> AppResult {
    if !session.workspace_config.daemon {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Unable to start, daemon has not been enabled")
                }
            }
        })?;

        return Ok(None);
    }

    let pid = session.get_daemon_connector()?.start_daemon().await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: format!("Daemon has been started with process ID {pid}")
                )
            }
        }
    })?;

    Ok(None)
}
