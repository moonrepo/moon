use miette::Diagnostic;
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

    #[diagnostic(code(daemon::client::rpc_call_failed))]
    #[error("Failed to make daemon RPC call.\n{}: {}", .error.code(), .error.message())]
    RpcFailed {
        #[source]
        error: Box<tonic::Status>,
    },
}
