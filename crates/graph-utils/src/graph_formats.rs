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
    graph: DiGraph<NodeIndex, &'graph E>,
    data: FxHashMap<NodeIndex, &'graph N>,
}

impl<N: Serialize, E: Serialize> GraphCache<'_, N, E> {
    /// Format graph as a JSON string.
    pub fn to_json(&self, pretty: bool) -> miette::Result<String> {
        Ok(json::format(self, pretty)?)
    }
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
        GraphCache {
            // Nodes are serialized by index, whatever the graph's node weights
            // are, as their data is serialized separately
            graph: self.get_graph().map(|index, _| index, |_, edge| edge),
            data: self.get_nodes(),
        }
        .to_json(pretty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Nodes stored separately, with each node's index as its weight
    struct IndexedGraph {
        graph: DiGraph<NodeIndex, String>,
        names: Vec<String>,
    }

    impl GraphData<String, String, String> for IndexedGraph {
        type NodeWeight = NodeIndex;

        fn get_graph(&self) -> &DiGraph<NodeIndex, String> {
            &self.graph
        }

        fn get_nodes(&self) -> FxHashMap<NodeIndex, &String> {
            self.graph
                .node_indices()
                .map(|index| (index, &self.names[index.index()]))
                .collect()
        }

        fn get_node_by_index(&self, index: &NodeIndex) -> &String {
            &self.names[index.index()]
        }

        fn get_node_key(&self, node: &String) -> String {
            node.clone()
        }
    }

    impl GraphToJson<String, String, String> for IndexedGraph {}

    // Nodes stored in the graph itself
    struct InlineGraph {
        graph: DiGraph<String, String>,
    }

    impl GraphData<String, String, String> for InlineGraph {
        type NodeWeight = String;

        fn get_graph(&self) -> &DiGraph<String, String> {
            &self.graph
        }

        fn get_nodes(&self) -> FxHashMap<NodeIndex, &String> {
            self.graph
                .node_indices()
                .map(|index| (index, &self.graph[index]))
                .collect()
        }

        fn get_node_by_index(&self, index: &NodeIndex) -> &String {
            &self.graph[*index]
        }

        fn get_node_key(&self, node: &String) -> String {
            node.clone()
        }
    }

    impl GraphToJson<String, String, String> for InlineGraph {}

    #[test]
    fn json_is_the_same_for_any_node_weight() {
        let names = ["a", "b", "c"].map(String::from);
        let edges = [(1, 0), (2, 0), (2, 1)];

        let mut indexed = IndexedGraph {
            graph: DiGraph::new(),
            names: names.to_vec(),
        };
        let mut inline = InlineGraph {
            graph: DiGraph::new(),
        };

        for (i, name) in names.iter().enumerate() {
            indexed.graph.add_node(NodeIndex::new(i));
            inline.graph.add_node(name.clone());
        }

        for (from, to) in edges {
            let (from, to) = (NodeIndex::new(from), NodeIndex::new(to));

            indexed.graph.add_edge(from, to, "dep".into());
            inline.graph.add_edge(from, to, "dep".into());
        }

        #[derive(Serialize)]
        struct RawCache<'graph> {
            graph: &'graph DiGraph<NodeIndex, String>,
            data: FxHashMap<NodeIndex, &'graph String>,
        }

        let json = indexed.to_json(false).unwrap();

        // Nodes are serialized by index, not by their data, the same as
        // serializing an indexed graph as is
        assert!(json.contains(r#""nodes":[0,1,2]"#), "{json}");
        assert_eq!(
            json,
            json::format(
                &RawCache {
                    graph: &indexed.graph,
                    data: indexed.get_nodes(),
                },
                false
            )
            .unwrap()
        );
        assert_eq!(json, inline.to_json(false).unwrap());
    }
}
