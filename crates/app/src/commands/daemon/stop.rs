use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use starbase::AppResult;

pub async fn stop(session: MoonSession) -> AppResult {
    let stopped = session.get_daemon_connector()?.stop_daemon().await?;

    if stopped {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Success) {
                    StyledText(content: "Daemon has been stopped")
                }
            }
        })?;
    } else {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Daemon is not running")
                }
            }
        })?;
    }

    Ok(None)
}
