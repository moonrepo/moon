use crate::session::MoonSession;
use crate::session::SessionResult;
use crate::watchers::WorkspaceWatcher;
use moon_daemon::{DaemonState, start_daemon_server};

fn install_daemon_panic_hook() {
    let previous_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        let message = if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            (*message).to_owned()
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            message.clone()
        } else {
            "non-string panic payload".to_owned()
        };
        let location = panic_info
            .location()
            .map(|location| {
                format!(
                    "{}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                )
            })
            .unwrap_or_else(|| "unknown".to_owned());

        tracing::error!(message, location, "Daemon server panicked");

        previous_hook(panic_info);
    }));
}

pub async fn server(session: MoonSession) -> SessionResult {
    install_daemon_panic_hook();

    start_daemon_server(
        DaemonState {
            app_context: session.get_app_context().await?,
            // Loaded in the background within the workspace watcher,
            // otherwise it causes this command to block for too long
            workspace_graph: Default::default(),
        },
        vec![Box::new(WorkspaceWatcher::new(session))],
    )
    .await?;

    Ok(None)
}
