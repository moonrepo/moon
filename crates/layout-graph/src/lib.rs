//! The layout graph: which toolchain dependencies root each project uses for
//! each of its toolchains, with which version and config, and what lives in
//! each root. See `docs/todo/LAYOUT_GRAPH.md` for the design.

mod layout_graph;
mod layout_graph_builder;

pub use layout_graph::*;
pub use layout_graph_builder::*;
