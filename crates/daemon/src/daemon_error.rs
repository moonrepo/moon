use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DaemonError {
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

    #[diagnostic(code(daemon::start_timed_out))]
    #[error("Timed out waiting for the daemon to start.")]
    StartTimedOut,

    #[diagnostic(code(daemon::stop_timed_out))]
    #[error("Timed out waiting for the daemon to stop gracefully.")]
    StopTimedOut,
}
