use crate::project_graph_error::ProjectGraphError;
use daggy::Dag;
use moon_common::Id;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_config::DependencyScope;
use moon_graph_utils::*;
use moon_project::Project;
use moon_project_expander::{ProjectExpander, ProjectExpanderContext};
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use scc::hash_map::Entry;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
pub struct ProjectNode {
    pub index: NodeIndex,
    pub project: Project,
}

#[derive(Debug, Default)]
pub struct ProjectGraph {
    pub context: GraphExpanderContext,

    /// Map of aliases to project IDs.
    pub aliases: FxHashMap<String, Id>,

    /// ID of the default project.
    pub default_id: Option<Id>,

    /// Directed-acyclic graph (DAG) of projects (by index) and their dependencies.
    pub graph: Dag<NodeIndex, DependencyScope>,

    /// Map of node indexes to project IDs.
    pub indexes: FxHashMap<NodeIndex, Id>,

    /// Map of project nodes by ID.
    pub nodes: FxHashMap<Id, ProjectNode>,

    /// Cache of file path lookups, mapped by starting path to project ID (as a string).
    fs_cache: Arc<scc::HashMap<PathBuf, Arc<String>>>,

    /// Map of expanded projects by ID.
    projects: Arc<scc::HashMap<Id, Arc<Project>>>,
}

impl ProjectGraph {
    pub fn new(context: GraphExpanderContext) -> Self {
        debug!("Creating project graph");

        Self {
            context,
            ..Default::default()
        }
    }

    /// Return a map of aliases to their project IDs. Projects without aliases are omitted.
    pub fn aliases(&self) -> FxHashMap<&str, &Id> {
        self.aliases
            .iter()
            .map(|(alias, id)| (alias.as_str(), id))
            .collect()
    }

    /// Return a project with the provided ID or alias from the graph.
    /// If the project does not exist or has been misconfigured, return an error.
    #[instrument(name = "get_project", skip(self))]
    pub fn get(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        self.internal_get(id_or_alias)
    }

    /// Return an unexpanded project with the provided ID or alias from the graph.
    pub fn get_unexpanded(&self, id_or_alias: &str) -> miette::Result<&Project> {
        let id = self.resolve_id(id_or_alias);

        let node = self
            .nodes
            .get(&id)
            .ok_or_else(|| ProjectGraphError::UnconfiguredID { id: id.to_string() })?;

        Ok(&node.project)
    }

    /// Return all projects from the graph.
    #[instrument(name = "get_all_projects", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Project>>> {
        let mut all = vec![];

        for id in self.nodes.keys() {
            all.push(self.internal_get(id)?);
        }

        Ok(all)
    }

    /// Return all unexpanded projects from the graph.
    pub fn get_all_unexpanded(&self) -> Vec<&Project> {
        self.nodes.values().map(|node| &node.project).collect()
    }

    /// Return the default project if it has been configured and exists.
    pub fn get_default(&self) -> miette::Result<Arc<Project>> {
        if let Some(id) = &self.default_id {
            return self.get(id);
        }

        Err(ProjectGraphError::NoDefaultProject.into())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    #[instrument(name = "get_project_from_path", skip(self))]
    pub fn get_from_path(&self, starting_file: Option<&Path>) -> miette::Result<Arc<Project>> {
        let current_file = starting_file.unwrap_or(&self.context.working_dir);

        let file = if current_file == self.context.workspace_root {
            Path::new(".")
        } else if let Ok(rel_file) = current_file.strip_prefix(&self.context.workspace_root) {
            rel_file
        } else {
            current_file
        };

        let id = self.internal_search(file)?;

        self.get(&id)
    }

    /// Return a map of project IDs to their file source paths.
    pub fn sources(&self) -> FxHashMap<&Id, &WorkspaceRelativePathBuf> {
        self.nodes
            .iter()
            .map(|(id, node)| (id, &node.project.source))
            .collect()
    }

    /// Focus the graph for a specific project by ID.
    pub fn focus_for(&self, id_or_alias: &Id, with_dependents: bool) -> miette::Result<Self> {
        let project = self.get(id_or_alias)?;
        let graph = self.to_focused_graph(&project, with_dependents);
        let (nodes, edges) = graph.into_nodes_edges();

        let mut dag = Dag::with_capacity(nodes.len(), edges.len());
        let mut indexes = FxHashMap::default();
        let mut projects = FxHashMap::default();

        // The focused graph has different node inndexes,
        // so we need to update our internal structures to match
        for (i, node) in nodes.into_iter().enumerate() {
            let new_index = NodeIndex::from(i as u32);
            let old_index = node.weight;
            let id = &self.indexes[&old_index];

            indexes.insert(new_index, id.to_owned());

            projects.insert(
                id.to_owned(),
                ProjectNode {
                    index: new_index,
                    project: self.get_node_by_index(&old_index).to_owned(),
                },
            );

            dag.add_node(new_index);
        }

        for edge in edges {
            dag.update_edge(edge.source(), edge.target(), edge.weight)
                .unwrap();
        }

        let aliases = self
            .aliases
            .iter()
            .filter_map(|(alias, id)| {
                if projects.contains_key(id) {
                    Some((alias.to_owned(), id.to_owned()))
                } else {
                    None
                }
            })
            .collect();

        Ok(Self {
            aliases,
            context: self.context.clone(),
            indexes,
            default_id: self.default_id.clone(),
            fs_cache: Arc::clone(&self.fs_cache),
            graph: dag,
            nodes: projects,
            projects: Arc::clone(&self.projects),
        })
    }

    fn internal_get(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        let id = self.resolve_id(id_or_alias);

        let project = match self.projects.entry_sync(id) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                let expander = ProjectExpander::new(ProjectExpanderContext {
                    aliases: self.aliases(),
                    workspace_root: &self.context.workspace_root,
                });

                let project = Arc::new(expander.expand(self.get_unexpanded(entry.key())?)?);

                entry.insert_entry(Arc::clone(&project));

                project
            }
        };

        Ok(project)
    }

    fn internal_search(&self, search: &Path) -> miette::Result<Arc<String>> {
        let cache_key = search.to_path_buf();

        let cache = match self.fs_cache.entry_sync(cache_key) {
            Entry::Occupied(entry) => Arc::clone(entry.get()),
            Entry::Vacant(entry) => {
                // Find the deepest matching path in case sub-projects are being used
                let mut remaining_length = 1000; // Start with a really fake number
                let mut possible_id = String::new();

                for (id, node) in &self.nodes {
                    if !search.starts_with(node.project.source.as_str()) {
                        continue;
                    }

                    if let Ok(diff) = search.relative_to(node.project.source.as_str()) {
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
                    return Err(ProjectGraphError::MissingFromPath {
                        dir: search.to_path_buf(),
                    }
                    .into());
                }

                let id = Arc::new(possible_id);

                entry.insert_entry(Arc::clone(&id));

                id
            }
        };

        Ok(cache)
    }

    pub fn resolve_id(&self, id_or_alias: &str) -> Id {
        Id::raw(if self.nodes.contains_key(id_or_alias) {
            id_or_alias
        } else if let Some(id) = self.aliases.get(id_or_alias) {
            id.as_str()
        } else {
            id_or_alias
        })
    }
}

impl GraphData<Project, DependencyScope, Id> for ProjectGraph {
    fn get_graph(&self) -> &DiGraph<NodeIndex, DependencyScope> {
        self.graph.graph()
    }

    fn get_nodes(&self) -> FxHashMap<NodeIndex, &Project> {
        self.nodes
            .values()
            .map(|node| (node.index, &node.project))
            .collect()
    }

    fn get_node_by_index(&self, index: &NodeIndex) -> &Project {
        &self.nodes[&self.indexes[index]].project
    }

    fn get_node_key(&self, node: &Project) -> Id {
        node.id.clone()
    }
}

impl GraphConnections<Project, DependencyScope, Id> for ProjectGraph {
    fn get_node_index(&self, node: &Project) -> NodeIndex {
        self.nodes[&node.id].index
    }
}

impl GraphConversions<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToDot<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToJson<Project, DependencyScope, Id> for ProjectGraph {}
