use miette::Diagnostic;
use moon_common::{Style, Stylize};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum ActionGraphError {
    #[diagnostic(code(action_graph::cycle_detected))]
    #[error("A dependency cycle has been detected for {}.", .0.style(Style::Label))]
    CycleDetected(String),

    #[diagnostic(code(action_graph::missing_toolchain_requirement))]
    #[error(
        "Toolchain {} requires the toolchain {}, but it has not been configured!",
        .id.style(Style::Id),
        .dep_id.style(Style::Id),
    )]
    MissingToolchainRequirement { id: String, dep_id: String },
}
