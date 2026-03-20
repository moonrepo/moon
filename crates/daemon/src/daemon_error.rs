use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DaemonError {
    #[diagnostic(code(daemon::connect_failed))]
    #[error("Failed to connect to the moon daemon at {endpoint}.")]
    ConnectFailed {
        endpoint: String,
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(daemon::not_running))]
    #[error("The moon daemon is not running.")]
    NotRunning,

    #[diagnostic(code(daemon::already_running))]
    #[error("The moon daemon is already running (pid {pid}).")]
    AlreadyRunning { pid: u32 },

    #[diagnostic(code(daemon::start_failed))]
    #[error("Failed to start the moon daemon.")]
    StartFailed {
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(daemon::stop_failed))]
    #[error("Failed to stop the moon daemon.")]
    StopFailed {
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(daemon::rpc_failed))]
    #[error("Failed to make daemon RPC call.\n{}: {}", .error.code(), .error.message())]
    RpcFailed {
        #[source]
        error: Box<tonic::Status>,
    },

    #[diagnostic(code(daemon::endpoint_bind_failed))]
    #[error("Failed to bind daemon endpoint at {endpoint}.")]
    EndpointBindFailed {
        endpoint: String,
        #[source]
        error: Box<std::io::Error>,
    },

    #[diagnostic(code(daemon::server_failed))]
    #[error("Daemon server encountered an error.")]
    ServerFailed {
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(daemon::start_timed_out))]
    #[error("Timed out waiting for the daemon to start.")]
    StartTimedOut,

    #[diagnostic(code(daemon::stop_timed_out))]
    #[error("Timed out waiting for the daemon to stop gracefully.")]
    StopTimedOut,
}
