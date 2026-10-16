mod action_graph;
mod action_graph_builder;
mod action_graph_error;
mod action_graph_options;
mod builder;

pub use action_graph::*;
pub use action_graph_error::*;
pub use action_graph_options::*;
pub use builder::*;
pub use moon_graph_utils::*;

/// The previous action graph builder, kept for the differential tests until
/// the new builder has fully replaced it.
pub mod legacy {
    pub use crate::action_graph_builder::{ActionGraphBuilder, RunTaskDependencies};
}
