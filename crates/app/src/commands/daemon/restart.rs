use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;

pub async fn restart(session: MoonSession) -> AppResult {
    if !session.workspace_config.daemon {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Unable to restart, daemon has not been enabled in configuration")
                }
            }
        })?;

        return Ok(None);
    }

    let connector = session.get_daemon_connector()?;
    let old_pid = connector.is_running();

    connector.stop_daemon().await?;

    let pid = connector.start_daemon().await?;
    let message = format!("Daemon has been restarted with process ID {pid}");

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: match old_pid {
                        Some(old) => format!("{message} (previous ID {old})"),
                        None => message
                    }
                )
            }
        }
    })?;

    Ok(None)
}
