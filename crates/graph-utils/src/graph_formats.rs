use crate::graph_traits::*;
use moon_common::is_test_env;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, NodeRef};
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[derive(Serialize)]
pub struct GraphCache<'graph, N, E> {
    graph: &'graph DiGraph<NodeIndex, E>,
    data: FxHashMap<NodeIndex, &'graph N>,
}

fn should_use_compact_view() -> bool {
    is_test_env() || cfg!(debug_assertions)
}

pub trait GraphToDot<N: Clone + Debug + Display, E: Clone + Debug + Display, K: Display + Hash + Eq>:
    GraphConversions<N, E, K>
{
    /// Format graph as a DOT string.
    fn to_dot(&self) -> String {
        let graph = self.to_labeled_graph();
        let dot = Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                let label = e.weight();
                let prefix = format!("label=\"{label}\"");

                if should_use_compact_view() {
                    prefix
                } else if e.source().index() == 0 {
                    format!("{prefix} arrowhead=none")
                } else {
                    format!("{prefix} arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let label = n.weight();
                let prefix = format!("label=\"{label}\"");

                if should_use_compact_view() {
                    prefix
                } else {
                    format!("{prefix} style=filled, shape=oval, fillcolor=gray, fontcolor=black")
                }
            },
        );

        format!("{dot:?}")
    }
}

pub trait GraphToJson<N: Serialize, E: Serialize, K>: GraphData<N, E, K> {
    /// Format graph as a JSON string.
    fn to_json(&self, pretty: bool) -> miette::Result<String> {
        Ok(json::format(
            &GraphCache {
                graph: self.get_graph(),
                data: self.get_nodes(),
            },
            pretty,
        )?)
    }
}
