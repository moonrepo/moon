use crate::session::MoonSession;
use crate::watchers::WorkspaceWatcher;
use moon_daemon::start_daemon_server;
use starbase::AppResult;

pub async fn server(session: MoonSession) -> AppResult {
    let connector = session.get_daemon_connector()?;
    let moon_version = session.cli_version.to_string();

    start_daemon_server(
        &connector.workspace_root,
        &connector.daemon_dir,
        &moon_version,
        session,
        vec![Box::new(WorkspaceWatcher::default())],
    )
    .await?;

    Ok(None)
}
