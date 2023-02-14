use moon_config::{ProjectID, ProjectsAliasesMap, ProjectsSourcesMap};
use moon_project::{Project, ProjectError};
use moon_utils::{get_workspace_root, path};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub type GraphType = DiGraph<Project, ()>;
pub type IndicesType = FxHashMap<ProjectID, NodeIndex>;

pub const LOG_TARGET: &str = "moon:project-graph";

#[derive(Serialize, Deserialize)]
pub struct ProjectGraph {
    /// Mapping of an alias to a project ID.
    pub aliases: ProjectsAliasesMap,

    /// Projects that have been loaded into scope represented as a DAG.
    graph: GraphType,

    /// Mapping of project IDs to node indices, as we need a way
    /// to query the graph by ID as it only supports it by index.
    indices: IndicesType,

    /// Mapping of project IDs to a relative file system location.
    /// Is the `projects` setting in `.moon/workspace.yml`.
    pub sources: ProjectsSourcesMap,
}

impl ProjectGraph {
    pub fn new(
        graph: GraphType,
        indices: IndicesType,
        sources: ProjectsSourcesMap,
        aliases: ProjectsAliasesMap,
    ) -> ProjectGraph {
        ProjectGraph {
            aliases,
            graph,
            indices,
            sources,
        }
    }

    /// Return a list of all configured project IDs in ascending order.
    pub fn ids(&self) -> Vec<ProjectID> {
        let mut nodes: Vec<ProjectID> = self.sources.keys().cloned().collect();
        nodes.sort();
        nodes
    }

    /// Return a project with the associated ID. If the project does not
    /// exist or has been misconfigured, return an error.
    pub fn get(&self, alias_or_id: &str) -> Result<&Project, ProjectError> {
        let id = match self.aliases.get(alias_or_id) {
            Some(project_id) => project_id,
            None => alias_or_id,
        };

        let index = self
            .indices
            .get(id)
            .ok_or_else(|| ProjectError::UnconfiguredID(id.to_owned()))?;

        Ok(self.graph.node_weight(*index).unwrap())
    }

    /// Return all projects from the graph.
    pub fn get_all(&self) -> Result<Vec<&Project>, ProjectError> {
        Ok(self.graph.raw_nodes().iter().map(|n| &n.weight).collect())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    #[track_caller]
    pub fn get_from_path<P: AsRef<Path>>(&self, current_file: P) -> Result<&Project, ProjectError> {
        let current_file = current_file.as_ref();
        let workspace_root = get_workspace_root();

        let file = if current_file == workspace_root {
            PathBuf::from(".")
        } else if current_file.starts_with(&workspace_root) {
            current_file
                .strip_prefix(&workspace_root)
                .unwrap()
                .to_path_buf()
        } else {
            current_file.to_path_buf()
        };

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = String::new();

        for (id, source) in &self.sources {
            if !file.starts_with(source) {
                continue;
            }

            if let Some(diff) = path::relative_from(&file, source) {
                let diff_string = path::to_string(diff)?;

                // Exact match, abort
                if diff_string.is_empty() {
                    possible_id = id.clone();
                    break;
                }

                if diff_string.len() < remaining_length {
                    remaining_length = diff_string.len();
                    possible_id = id.clone();
                }
            }
        }

        if possible_id.is_empty() {
            return Err(ProjectError::MissingProjectFromPath(file));
        }

        self.get(&possible_id)
    }

    /// Return a list of direct project IDs that the defined project depends on.
    pub fn get_dependencies_of(&self, project: &Project) -> Result<Vec<ProjectID>, ProjectError> {
        let deps = self
            .graph
            .neighbors_directed(*self.indices.get(&project.id).unwrap(), Direction::Outgoing)
            .map(|idx| self.graph.node_weight(idx).unwrap().id.clone())
            .collect();

        Ok(deps)
    }

    /// Return a list of project IDs that require the defined project.
    pub fn get_dependents_of(&self, project: &Project) -> Result<Vec<ProjectID>, ProjectError> {
        let deps = self
            .graph
            .neighbors_directed(*self.indices.get(&project.id).unwrap(), Direction::Incoming)
            .map(|idx| self.graph.node_weight(idx).unwrap().id.clone())
            .collect();

        Ok(deps)
    }

    /// Get a labelled representation of the dep graph (which can be serialized easily).
    pub fn labeled_graph(&self) -> DiGraph<String, ()> {
        let graph = self.graph.clone();
        graph.map(|_, n| n.id.clone(), |_, e| *e)
    }

    /// Format as a DOT string.
    pub fn to_dot(&self) -> String {
        let labeled_graph = self.graph.map(|_, n| n.id.clone(), |_, e| e);
        // let highlight_id = highlight_id.clone().unwrap_or_default();

        let dot = Dot::with_attr_getters(
            &labeled_graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, e| {
                if e.source().index() == 0 {
                    String::from("arrowhead=none")
                } else {
                    String::from("arrowhead=box, arrowtail=box")
                }
            },
            &|_, n| {
                let id = n.1;

                // if id == &highlight_id {
                // //     String::from("style=filled, shape=circle, fillcolor=palegreen, fontcolor=black")
                // } else {
                format!("label=\"{id}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black")
                // }
            },
        );

        format!("{dot:?}")
    }
}
