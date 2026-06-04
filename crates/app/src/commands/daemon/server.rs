use crate::session::MoonSession;
use crate::watchers::WorkspaceWatcher;
use moon_daemon::{DaemonState, start_daemon_server};
use starbase::AppResult;

pub async fn server(session: MoonSession) -> AppResult {
    start_daemon_server(
        DaemonState {
            app_context: session.get_app_context().await?,
            // Loaded in the background with the workspace watcher,
            // otherwise it causes this command to block
            workspace_graph: Default::default(),
        },
        vec![Box::new(WorkspaceWatcher::new(session))],
    )
    .await?;

    Ok(None)
}
