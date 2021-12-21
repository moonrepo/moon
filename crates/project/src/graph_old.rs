use crate::project::ProjectsMap;
use itertools::Itertools;
use petgraph::dot::{Config, Dot};
use petgraph::graphmap::DiGraphMap;
use petgraph::{Directed, Graph};
use std::collections::HashMap;

pub type ProjectGraphType<'g> = Graph<&'g str, (), Directed>;

#[derive(Debug)]
pub struct ProjectGraph<'g> {
    graph: ProjectGraphType<'g>,
}

impl<'g> ProjectGraph<'g> {
    pub fn create(projects: &'g ProjectsMap) -> ProjectGraph<'g> {
        let mut graph: ProjectGraphType<'g> = Graph::new();
        let mut indices = HashMap::new(); // project.id -> node indices

        let root_node = graph.add_node("(root)");

        // println!("root_node = {:#?}", root_node);

        // Map every project to a node
        for id in projects.keys().sorted() {
            indices.insert(id, graph.add_node(id.as_str()));
        }

        // println!("indices = {:#?}", indices);
        // println!("graph = {:#?}", graph);

        // Link dependencies between project nodes with an edge
        let get_node_index = |id: &String| indices.get(id).unwrap();

        for id in projects.keys().sorted() {
            graph.add_edge(root_node, *get_node_index(id), ());

            let project = projects.get(id).unwrap();

            if project.config.is_some() {
                let config = project.config.as_ref().unwrap();

                if config.depends_on.is_some() {
                    for dep in config.depends_on.as_ref().unwrap() {
                        println!("{:?} => {:?}", id, dep);
                        // println!("{:?} => {:?}", get_node_index(id), get_node_index(dep));
                        graph.add_edge(*get_node_index(id), *get_node_index(dep), ());
                        // graph.add_edge(get_node_index(dep), get_node_index(id), ());
                        println!("graph = {:#?}", graph);
                    }
                }
            }
        }

        // graph.add_edge(
        //     get_node_index(&"bar".to_owned()),
        //     get_node_index(&"basic".to_owned()),
        //     (),
        // );

        // println!("graph = {:#?}", graph);

        ProjectGraph { graph }
    }

    pub fn to_dot(&self) -> String {
        println!(
            "{:#?}",
            Dot::with_config(&self.graph, &[Config::EdgeNoLabel])
        );

        let mut g = DiGraphMap::new();
        g.add_edge("x", "y", -1);

        println!("{:#?}", Dot::with_config(&g, &[Config::EdgeNoLabel]));

        // format!(
        //     "{:?}",
        //     Dot::with_config(
        //         &self.graph,
        //         &[Config::EdgeNoLabel]
        //     )
        // )

        String::from("")
    }
}
