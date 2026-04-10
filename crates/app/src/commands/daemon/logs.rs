use crate::session::MoonSession;
use iocraft::prelude::element;
use moon_console::ui::{Container, Notice, StyledText, Variant};
use moon_process::Command;
use starbase::AppResult;
use std::path::Path;

pub async fn logs(session: MoonSession) -> AppResult {
    let connector = session.get_daemon_connector()?;
    let log_path = connector.get_log_file();

    if connector.is_running().is_none() || !log_path.exists() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Caution) {
                    StyledText(content: "Daemon is not running")
                }
            }
        })?;

        return Ok(None);
    }

    tail_logs(&session, &log_path).await?;

    Ok(None)
}

#[cfg(unix)]
async fn tail_logs(session: &MoonSession, log_path: &Path) -> AppResult {
    use moon_process::find_command_on_path;

    if find_command_on_path("tail".into()).is_none() {
        session.console.render(element! {
            Container {
                Notice(variant: Variant::Failure) {
                    StyledText(content: "The <shell>tail</shell> command is required to view logs")
                }
            }
        })?;

        return Ok(Some(1));
    }

    Command::new("tail")
        .arg("-f")
        .arg(log_path)
        .exec_stream_output()
        .await?;

    Ok(None)
}

#[cfg(windows)]
async fn tail_logs(_session: &MoonSession, log_path: &Path) -> AppResult {
    use moon_process::get_default_shell;

    Command::new("Get-Content")
        .arg(log_path)
        .arg("-Wait")
        // Must run in powershell
        .set_shell(get_default_shell())
        .exec_stream_output()
        .await?;

    Ok(None)
}
