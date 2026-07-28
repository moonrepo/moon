use miette::Diagnostic;
use std::error::Error as StdError;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum DaemonClientError {
    #[diagnostic(code(daemon::client::connect_failed))]
    #[error("Failed to connect to the moon daemon at {endpoint}.")]
    ConnectFailed {
        endpoint: String,
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(daemon::client::connect_timed_out))]
    #[error("Timed out connecting to the moon daemon at {endpoint} after {timeout_secs} seconds.")]
    ConnectTimedOut { endpoint: String, timeout_secs: u64 },

    #[diagnostic(code(daemon::client::rpc_call_failed))]
    #[error("Failed to make daemon RPC call.\n{}: {}", .error.code(), .error.message())]
    RpcFailed {
        #[source]
        error: Box<tonic::Status>,
    },

    #[diagnostic(code(daemon::client::rpc_call_timed_out))]
    #[error("Daemon {procedure} call timed out after {timeout_secs} seconds.")]
    RpcTimedOut {
        procedure: String,
        timeout_secs: u64,
    },
}

impl DaemonClientError {
    /// Return true if the failure indicates the daemon endpoint is not
    /// currently accepting connections — missing, refusing, or timing out —
    /// which can mean the daemon is still starting up, restarting, or gone.
    /// Callers may retry these failures; other errors are not retryable.
    pub fn is_endpoint_unavailable(&self) -> bool {
        match self {
            Self::ConnectTimedOut { .. } => true,
            Self::ConnectFailed { error, .. } => {
                find_io_error(error.as_ref()).is_some_and(|io_error| {
                    matches!(
                        io_error.kind(),
                        IoErrorKind::NotFound
                            | IoErrorKind::ConnectionRefused
                            | IoErrorKind::TimedOut
                    )
                })
            }
            _ => false,
        }
    }
}

/// Walk an error's source chain looking for the underlying I/O error.
/// tonic wraps connector failures in transport/hyper layers, so the
/// interesting `ErrorKind` is buried a few levels deep.
fn find_io_error<'error>(error: &'error (dyn StdError + 'static)) -> Option<&'error IoError> {
    let mut source: Option<&(dyn StdError + 'static)> = Some(error);

    while let Some(current) = source {
        if let Some(io_error) = current.downcast_ref::<IoError>() {
            return Some(io_error);
        }

        source = current.source();
    }

    None
}
