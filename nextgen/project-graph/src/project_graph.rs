use moon_common::Id;
use moon_config::DependencyScope;
use moon_project::Project;
use moon_project_builder::ProjectBuilderError;
use pathdiff::diff_paths;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use std::path::Path;

pub type GraphType = DiGraph<Project, DependencyScope>;

pub struct ProjectNode {
    alias: Option<String>,
    index: NodeIndex,
}

pub struct ProjectGraph {
    graph: GraphType,
    nodes: FxHashMap<Id, ProjectNode>,
}

impl ProjectGraph {
    // TODO new, query

    /// Return a list of project IDs that the provide project depends on.
    pub fn dependencies_of(&self, project: &Project) -> miette::Result<Vec<&Id>> {
        let deps = self
            .graph
            .neighbors_directed(
                self.nodes.get(&project.id).unwrap().index,
                Direction::Outgoing,
            )
            .map(|idx| &self.graph.node_weight(idx).unwrap().id)
            .collect();

        Ok(deps)
    }

    /// Return a list of project IDs that require the provided project.
    pub fn dependents_of(&self, project: &Project) -> miette::Result<Vec<&Id>> {
        let deps = self
            .graph
            .neighbors_directed(
                self.nodes.get(&project.id).unwrap().index,
                Direction::Incoming,
            )
            .map(|idx| &self.graph.node_weight(idx).unwrap().id)
            .collect();

        Ok(deps)
    }

    /// Return a project with the provided ID or alias from the graph.
    /// If the project does not exist or has been misconfigured, return an error.
    pub fn get(&self, alias_or_id: &str) -> miette::Result<&Project> {
        let id = if self.nodes.contains_key(alias_or_id) {
            alias_or_id
        } else {
            self.nodes
                .iter()
                .find(|(_, node)| node.alias.as_ref().is_some_and(|a| a == alias_or_id))
                .map(|(id, _)| id.as_str())
                .unwrap_or(alias_or_id)
        };

        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| ProjectBuilderError::UnconfiguredID(Id::raw(id)))?;

        Ok(self.graph.node_weight(node.index).unwrap())
    }

    /// Return all projects from the graph.
    pub fn get_all(&self) -> miette::Result<Vec<&Project>> {
        Ok(self.graph.raw_nodes().iter().map(|n| &n.weight).collect())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    pub fn get_from_path<P: AsRef<Path>, R: AsRef<Path>>(
        &self,
        starting_file: P,
        workspace_root: R,
    ) -> miette::Result<&Project> {
        let current_file = starting_file.as_ref();
        let workspace_root = workspace_root.as_ref();

        let file = if current_file == workspace_root {
            Path::new(".")
        } else if let Ok(rel_file) = current_file.strip_prefix(&workspace_root) {
            rel_file
        } else {
            current_file
        };

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = "";

        for project in self.get_all()? {
            if !file.starts_with(project.source.as_str()) {
                continue;
            }

            if let Some(diff) = diff_paths(file, project.source.as_str()) {
                let diff_comps = diff.components().count();

                // Exact match, abort
                if diff_comps == 0 {
                    possible_id = project.id.as_str();
                    break;
                }

                if diff_comps < remaining_length {
                    remaining_length = diff_comps;
                    possible_id = project.id.as_str();
                }
            }
        }

        if possible_id.is_empty() {
            return Err(ProjectBuilderError::MissingFromPath(file.to_path_buf()).into());
        }

        self.get(possible_id)
    }

    /// Return a list of IDs for all projects currently within the graph.
    pub fn ids(&self) -> Vec<&Id> {
        self.graph
            .raw_nodes()
            .iter()
            .map(|n| &n.weight.id)
            .collect()
    }

    /// Get a labelled representation of the graph (which can be serialized easily).
    pub fn labeled_graph(&self) -> DiGraph<String, DependencyScope> {
        self.graph.map(|_, n| n.id.to_string(), |_, e| *e)
    }

    /// Format graph as a DOT string.
    pub fn to_dot(&self) -> String {
        let dot = Dot::with_attr_getters(
            &self.graph,
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
                let label = &n.1.id;

                format!(
                    "label=\"{label}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black"
                )
            },
        );

        format!("{dot:?}")
    }
}
