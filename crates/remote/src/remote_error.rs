use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RemoteError {
    #[diagnostic(code(remote::grpc::call_failed))]
    #[error("Failed to make gRPC call.")]
    CallFailed {
        #[source]
        error: Box<tonic::Status>,
    },

    #[diagnostic(code(remote::grpc::call_failed))]
    #[error("Failed to make gRPC call: {error}")]
    CallFailedViaSource { error: String },

    #[diagnostic(code(remote::grpc::connect_failed))]
    #[error("Failed to connect to gRPC host.")]
    ConnectFailed {
        #[source]
        error: Box<tonic::transport::Error>,
    },

    #[diagnostic(code(remote::http::no_support))]
    #[error("The HTTP based remote service is currently not supported, use gRPC instead.")]
    NoHttpClient,

    #[diagnostic(code(remote::unsupported_protocol))]
    #[error("Unknown remote host protocol, only gRPC is supported.")]
    UnknownHostProtocol,
}
