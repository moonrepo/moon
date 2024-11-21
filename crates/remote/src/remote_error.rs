use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RemoteError {
    #[error("Failed to make gRPC call: {}", .0.to_string().style(Style::MutedLight))]
    CallFailed(Box<tonic::Status>),

    #[error("Failed to connect to gRPC host: {}", .0.to_string().style(Style::MutedLight))]
    ConnectFailed(Box<tonic::transport::Error>),

    #[error("The HTTP based remote service is currently not supported, use gRPC instead.")]
    NoHttpClient,

    #[error("Unknown remote host protocol, only gRPC is supported.")]
    UnknownHostProtocol,
}
