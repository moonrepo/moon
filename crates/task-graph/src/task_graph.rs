use moon_target::Target;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use tracing::debug;

pub type GraphType = DiGraph<Target, ()>;

#[derive(Serialize)]
pub struct TaskGraphCache<'graph> {
    graph: &'graph GraphType,
}

#[derive(Default)]
pub struct TaskGraph {
    /// Directed-acyclic graph (DAG) of targets and their relationships.
    graph: GraphType,

    /// Mapping of task targets to graph node indices.
    nodes: FxHashMap<Target, NodeIndex>,
}

impl TaskGraph {
    pub fn new(graph: GraphType, nodes: FxHashMap<Target, NodeIndex>) -> Self {
        debug!("Creating task graph");

        Self { graph, nodes }
    }

    pub fn dependencies_of(&self, target: &Target) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(*self.nodes.get(target).unwrap(), Direction::Outgoing)
            .map(|idx| self.graph.node_weight(idx).unwrap())
            .collect();

        Ok(deps)
    }

    pub fn dependents_of(&self, target: &Target) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(*self.nodes.get(target).unwrap(), Direction::Incoming)
            .map(|idx| self.graph.node_weight(idx).unwrap())
            .collect();

        Ok(deps)
    }

    /// Return a list of targets for all tasks currently within the graph.
    pub fn targets(&self) -> Vec<&Target> {
        self.graph.raw_nodes().iter().map(|n| &n.weight).collect()
    }

    /// Get a labelled representation of the graph (which can be serialized easily).
    pub fn labeled_graph(&self) -> DiGraph<String, ()> {
        self.graph.map(|_, n| n.id.clone(), |_, e| *e)
    }

    /// Format graph as a DOT string.
    pub fn to_dot(&self) -> String {
        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                if e.source().index() == 0 {
                    format!("arrowhead=none")
                } else {
                    format!("arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let label = &n.1.id;

                format!(
                    "label=\"{label}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black"
                )
            },
        );

        format!("{dot:?}")
    }

    /// Format graph as a JSON string.
    pub fn to_json(&self) -> miette::Result<String> {
        Ok(json::format(&TaskGraphCache { graph: &self.graph }, true)?)
    }
}
