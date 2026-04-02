use crate::session::MoonSession;
use crate::watchers::WorkspaceWatcher;
use moon_daemon::{DaemonState, start_daemon_server};
use starbase::AppResult;

pub async fn server(session: MoonSession) -> AppResult {
    start_daemon_server(
        DaemonState {
            app_context: session.get_app_context().await?,
            workspace_graph: session.get_workspace_graph().await?,
        },
        vec![Box::new(WorkspaceWatcher::new(session))],
    )
    .await?;

    Ok(None)
}
