use crate::errors::ProjectError;
use crate::ProjectsMap;
use petgraph::prelude::*;
use petgraph::{Graph, Undirected};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ProjectGraph<'a> {
    graph: Graph<&'a str, (), Undirected>,

    indices: HashMap<&'a str, NodeIndex>,
}

impl<'a> ProjectGraph<'a> {
    pub fn new(projects: &'a ProjectsMap) -> Result<ProjectGraph<'a>, ProjectError> {
        let mut graph = ProjectGraph {
            graph: Graph::new_undirected(),
            indices: HashMap::new(),
        };

        // Map every project to a node
        for id in projects.keys() {
            graph
                .indices
                .insert(id.as_str(), graph.graph.add_node(id.as_str()));
        }

        // Link dependencies between projects with an edge
        for (id, project) in projects {
            if project.config.is_some() {
                let config = project.config.as_ref().unwrap();

                if config.depends_on.is_some() {
                    for dep in config.depends_on.as_ref().unwrap() {
                        graph.graph.add_edge(
                            graph.get_project_index(id),
                            graph.get_project_index(dep),
                            (),
                        );
                    }
                }
            }
        }

        Ok(graph)
    }

    fn get_project_index(&self, id: &str) -> NodeIndex {
        *self.indices.get(id).unwrap()
    }
}
