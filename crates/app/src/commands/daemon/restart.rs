use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;

pub async fn restart(session: MoonSession) -> AppResult {
    let connector = session.get_daemon_connector()?;
    let old_pid = connector.is_already_running();

    connector.stop_daemon().await?;

    let pid = connector.start_daemon().await?;

    session.console.render(element! {
        Container {
            Notice(variant: Variant::Success) {
                StyledText(
                    content: match old_pid {
                        Some(old) => format!("Daemon has been restarted with process ID {pid} (previous ID {old})"),
                        None => format!("Daemon has been restarted with process ID {pid}")
                    }
                )
            }
        }
    })?;

    Ok(None)
}
