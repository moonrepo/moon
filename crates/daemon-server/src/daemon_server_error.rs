use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DaemonServerError {
    #[diagnostic(code(daemon::server::endpoint_bind_failed))]
    #[error("Failed to bind daemon endpoint at {endpoint}.")]
    EndpointBindFailed {
        endpoint: String,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(daemon::server::server_failed))]
    #[error("Daemon server encountered an error.")]
    ServerFailed {
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(daemon::server::watcher_failed))]
    #[error("File watcher failed to start.")]
    WatcherFailed {
        #[source]
        error: Box<notify_debouncer_full::notify::Error>,
    },
}
