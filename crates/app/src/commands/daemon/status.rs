use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::*;
use moon_time::elapsed;
use starbase::AppResult;
use std::time::Duration;

pub async fn status(session: MoonSession) -> AppResult {
    let connector = session.get_daemon_connector()?;

    if connector.is_running().is_none() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Daemon is not running")
                }
            }
        })?;

        return Ok(None);
    }

    let status = connector.connect().await?.status().await?;

    session.console.render(element! {
        Container {
            Section(title: "Process") {
                Entry(
                    name: "PID",
                    value: element! {
                        StyledText(
                            content: status.pid.to_string(),
                            style: Style::Id
                        )
                    }.into_any()
                )
                Entry(
                    name: if cfg!(unix) {
                        "Socket"
                    } else {
                        "Named pipe"
                    },
                    value: element! {
                        StyledText(
                            content: status.endpoint,
                            style: Style::File
                        )
                    }.into_any()
                )
                Entry(
                    name: "Uptime",
                    content: elapsed(Duration::from_secs(status.uptime_secs)),
                )
            }

            Section(title: "Paths") {
                Entry(
                    name: "PID file",
                    value: element! {
                        StyledText(
                            content: connector.get_pid_file().to_string_lossy(),
                            style: Style::Path
                        )
                    }.into_any()
                )
                Entry(
                    name: "Log file",
                    value: element! {
                        StyledText(
                            content: connector.get_log_file().to_string_lossy(),
                            style: Style::Path
                        )
                    }.into_any()
                )
            }
        }
    })?;

    Ok(None)
}
