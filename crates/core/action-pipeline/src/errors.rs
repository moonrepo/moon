use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum PipelineError {
    #[error("{0}")]
    Aborted(String),

    #[error("An unknown action was encountered in the pipeline. Unable to proceed!")]
    UnknownActionNode,
}
