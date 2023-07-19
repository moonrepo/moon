use crate::project_graph_error::ProjectGraphError;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::DependencyScope;
use moon_project::Project;
use moon_task_expander::TasksExpander;
use pathdiff::diff_paths;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type GraphType = DiGraph<Project, DependencyScope>;
pub type ProjectsCache = FxHashMap<Id, Arc<Project>>;

#[derive(Default)]
pub struct ProjectNode {
    pub alias: Option<String>,
    pub index: NodeIndex,
    pub source: WorkspaceRelativePathBuf,
}

#[derive(Default)]
pub struct ProjectGraph {
    /// Direct-acyclic graph of non-expanded projects and their dependencies.
    pub graph: GraphType,

    /// Graph node information, mapped by project ID.
    pub nodes: FxHashMap<Id, ProjectNode>,

    /// Expanded projects, mapped by project ID.
    pub projects: Arc<RwLock<ProjectsCache>>,

    /// Workspace root, required for expansion.
    pub workspace_root: PathBuf,
}

impl ProjectGraph {
    // TODO query

    pub fn new(graph: GraphType, nodes: FxHashMap<Id, ProjectNode>, workspace_root: &Path) -> Self {
        Self {
            graph,
            nodes,
            projects: Arc::new(RwLock::new(FxHashMap::default())),
            workspace_root: workspace_root.to_owned(),
        }
    }

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
    pub fn get(&self, alias_or_id: &str) -> miette::Result<Arc<Project>> {
        let id = if self.nodes.contains_key(alias_or_id) {
            alias_or_id
        } else {
            self.nodes
                .iter()
                .find(|(_, node)| node.alias.as_ref().is_some_and(|a| a == alias_or_id))
                .map(|(id, _)| id.as_str())
                .unwrap_or(alias_or_id)
        };

        // Check if the expanded project has been created, if so return it
        {
            if let Some(project) = self.read_cache().get(id) {
                return Ok(project.clone());
            }
        }

        // Otherwise clone and expand the project
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| ProjectGraphError::UnconfiguredID(Id::raw(id)))?;

        let mut project = self.graph.node_weight(node.index).unwrap().clone();

        TasksExpander::expand(&mut project, &self.workspace_root, |_| Ok(vec![]))?;

        // And then cache it with an Arc, allowing for reuse
        {
            self.write_cache()
                .insert(project.id.clone(), Arc::new(project));
        }

        Ok(self.read_cache().get(id).unwrap().clone())
    }

    /// Return all projects from the graph.
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Project>>> {
        let mut all = vec![];

        for id in self.nodes.keys() {
            all.push(self.get(id)?);
        }

        Ok(all)
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    pub fn get_from_path<P: AsRef<Path>>(&self, starting_file: P) -> miette::Result<Arc<Project>> {
        let current_file = starting_file.as_ref();

        let file = if current_file == self.workspace_root {
            Path::new(".")
        } else if let Ok(rel_file) = current_file.strip_prefix(&self.workspace_root) {
            rel_file
        } else {
            current_file
        };

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = String::new();

        for (id, node) in &self.nodes {
            if !file.starts_with(node.source.as_str()) {
                continue;
            }

            if let Some(diff) = diff_paths(file, node.source.as_str()) {
                let diff_comps = diff.components().count();

                // Exact match, abort
                if diff_comps == 0 {
                    possible_id = id.as_str().to_owned();
                    break;
                }

                if diff_comps < remaining_length {
                    remaining_length = diff_comps;
                    possible_id = id.as_str().to_owned();
                }
            }
        }

        if possible_id.is_empty() {
            return Err(ProjectGraphError::MissingFromPath(file.to_path_buf()).into());
        }

        self.get(&possible_id)
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

    fn read_cache(&self) -> RwLockReadGuard<ProjectsCache> {
        self.projects
            .read()
            .expect("Failed to acquire read access to project graph!")
    }

    fn write_cache(&self) -> RwLockWriteGuard<ProjectsCache> {
        self.projects
            .write()
            .expect("Failed to acquire write access to project graph!")
    }
}

#[cfg(test)]
mod tests {
    use crate::ProjectGraph;

    #[test]
    fn wat() {
        let graph = ProjectGraph::default();
        let project = graph.get("foo").unwrap();

        project.get_dependency_ids();

        let project = graph.get("foo").unwrap();

        project.get_dependency_ids();
    }
}
