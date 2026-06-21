use crate::session::{MoonSession, SessionResult};
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};

pub async fn start(session: MoonSession) -> SessionResult {
    if !session.workspace_config.daemon {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Unable to start, daemon has not been enabled in configuration")
                }
            }
        })?;

        return Ok(None);
    }

    let pid = session
        .get_daemon_connector()?
        .start_daemon(true)
        .await?
        .unwrap_or_default();

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
