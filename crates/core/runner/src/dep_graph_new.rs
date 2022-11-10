use crate::errors::DepGraphError;
use moon_action::ActionNode;
use moon_logger::{color, debug, map_list, trace};
use moon_platform::Runtime;
use moon_project::Project;
use moon_project_graph::ProjectGraph;
use moon_task::{Target, TargetError, TargetProjectScope, TouchedFilePaths};
use petgraph::algo::toposort;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;
use petgraph::Graph;
use rustc_hash::{FxHashMap, FxHashSet};

pub use petgraph::graph::NodeIndex;

const LOG_TARGET: &str = "moon:dep-graph";

pub type DepGraphType = DiGraph<ActionNode, ()>;
pub type BatchedTopoSort = Vec<Vec<NodeIndex>>;

/// A directed acyclic graph (DAG) for the work that needs to be processed, based on a
/// project or task's dependency chain. This is also known as a "task graph" (not to
/// be confused with ours) or a "dependency graph".
pub struct DepGraph<'l> {
    pub graph: DepGraphType,

    indices: FxHashMap<ActionNode, NodeIndex>,

    project_graph: &'l ProjectGraph,
}

impl<'l> DepGraph<'l> {
    pub fn new(project_graph: &'l ProjectGraph) -> Self {
        debug!(target: LOG_TARGET, "Creating dependency graph",);

        DepGraph {
            graph: Graph::new(),
            indices: FxHashMap::default(),
            project_graph,
        }
    }
}
