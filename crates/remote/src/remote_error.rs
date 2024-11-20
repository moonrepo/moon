use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RemoteError {
    #[error("Failed to make gRPC call: {0}")]
    Tonic(#[from] Box<tonic::Status>),
}
