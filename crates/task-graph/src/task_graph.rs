use moon_target::Target;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use tracing::debug;

pub type TaskGraphType = DiGraph<Target, ()>;

#[derive(Serialize)]
pub struct TaskGraphCache<'graph> {
    graph: &'graph TaskGraphType,
}

#[derive(Clone, Debug, Default)]
pub struct TaskNode {
    pub index: NodeIndex,
}

#[derive(Default)]
pub struct TaskGraph {
    /// Directed-acyclic graph (DAG) of targets and their relationships.
    graph: TaskGraphType,

    /// Mapping of task targets to graph node indices.
    nodes: FxHashMap<Target, TaskNode>,
}

impl TaskGraph {
    pub fn new(graph: TaskGraphType, nodes: FxHashMap<Target, TaskNode>) -> Self {
        debug!("Creating task graph");

        Self { graph, nodes }
    }

    /// Return a list of targets that the provided target depends on.
    pub fn dependencies_of(&self, target: &Target) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(self.nodes.get(target).unwrap().index, Direction::Outgoing)
            .map(|idx| self.graph.node_weight(idx).unwrap())
            .collect();

        Ok(deps)
    }

    /// Return a list of targets that require the provided target.
    pub fn dependents_of(&self, target: &Target) -> miette::Result<Vec<&Target>> {
        let deps = self
            .graph
            .neighbors_directed(self.nodes.get(target).unwrap().index, Direction::Incoming)
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
        self.graph.map(|_, n| n.to_string(), |_, e| *e)
    }

    /// Format graph as a DOT string.
    pub fn to_dot(&self) -> String {
        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                if e.source().index() == 0 {
                    "arrowhead=none".into()
                } else {
                    "arrowhead=box, arrowtail=box".into()
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
