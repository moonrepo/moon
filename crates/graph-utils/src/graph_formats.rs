use crate::graph_traits::*;
use petgraph::dot::{Config, Dot};
use petgraph::graph::DiGraph;
use petgraph::visit::{EdgeRef, NodeRef};
use serde::Serialize;
use starbase_utils::json;
use std::fmt::{Debug, Display};

#[derive(Serialize)]
pub struct GraphCache<'graph, N, E> {
    graph: &'graph DiGraph<N, E>,
    // data: &'graph FxHashMap<K, N>,
}

pub trait GraphToDot<N: Debug + Display, E: Debug + Display, K: Display>:
    GraphData<N, E, K>
{
    /// Format graph as a DOT string.
    fn to_dot(&self) -> String {
        let dot = Dot::with_attr_getters(
            self.get_graph(),
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                let label = e.weight().to_string();

                if e.source().index() == 0 {
                    format!("label=\"{label}\" arrowhead=none")
                } else {
                    format!("label=\"{label}\" arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let label = n.weight().to_string();

                format!(
                    "label=\"{label}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black"
                )
            },
        );

        format!("{dot:?}")
    }
}

pub trait GraphToJson<N: Serialize, E: Serialize, K>: GraphData<N, E, K> {
    /// Format graph as a JSON string.
    fn to_json(&self) -> miette::Result<String> {
        Ok(json::format(
            &GraphCache {
                graph: self.get_graph(),
                // data: self.get_nodes(),
            },
            true,
        )?)
    }
}
