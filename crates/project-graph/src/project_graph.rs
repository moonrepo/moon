use crate::project_graph_error::ProjectGraphError;
use moon_common::Id;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_config::DependencyScope;
use moon_graph_utils::*;
use moon_project::Project;
use moon_project_expander::{ProjectExpander, ProjectExpanderContext};
use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use scc::HashMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, instrument};

pub type ProjectGraphType = DiGraph<Project, DependencyScope>;
pub type ProjectsCache = FxHashMap<Id, Arc<Project>>;

#[derive(Clone, Debug, Default)]
pub struct ProjectMetadata {
    pub alias: Option<String>,
    pub index: NodeIndex,
    pub original_id: Option<Id>,
    pub source: WorkspaceRelativePathBuf,
}

impl ProjectMetadata {
    pub fn new(index: usize) -> Self {
        ProjectMetadata {
            index: NodeIndex::new(index),
            ..ProjectMetadata::default()
        }
    }
}

#[derive(Default)]
pub struct ProjectGraph {
    context: GraphExpanderContext,

    /// Cache of file path lookups, mapped by starting path to project ID (as a string).
    fs_cache: HashMap<PathBuf, Arc<String>>,

    /// Directed-acyclic graph (DAG) of non-expanded projects and their dependencies.
    graph: ProjectGraphType,

    /// Project metadata, mapped by project ID.
    metadata: FxHashMap<Id, ProjectMetadata>,

    /// Expanded projects, mapped by project ID.
    projects: Arc<RwLock<ProjectsCache>>,
}

impl ProjectGraph {
    pub fn new(
        graph: ProjectGraphType,
        metadata: FxHashMap<Id, ProjectMetadata>,
        context: GraphExpanderContext,
    ) -> Self {
        debug!("Creating project graph");

        Self {
            context,
            graph,
            metadata,
            projects: Arc::new(RwLock::new(FxHashMap::default())),
            fs_cache: HashMap::new(),
        }
    }

    /// Return a map of aliases to their project IDs. Projects without aliases are omitted.
    pub fn aliases(&self) -> FxHashMap<&str, &Id> {
        self.metadata
            .iter()
            .filter_map(|(id, metadata)| metadata.alias.as_ref().map(|alias| (alias.as_str(), id)))
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

        let metadata = self
            .metadata
            .get(&id)
            .ok_or(ProjectGraphError::UnconfiguredID(id))?;

        Ok(self.graph.node_weight(metadata.index).unwrap())
    }

    /// Return all projects from the graph.
    #[instrument(name = "get_all_projects", skip(self))]
    pub fn get_all(&self) -> miette::Result<Vec<Arc<Project>>> {
        let mut all = vec![];

        for id in self.metadata.keys() {
            all.push(self.internal_get(id)?);
        }

        Ok(all)
    }

    /// Return all unexpanded projects from the graph.
    pub fn get_all_unexpanded(&self) -> Vec<&Project> {
        self.graph
            .raw_nodes()
            .iter()
            .map(|node| &node.weight)
            .collect()
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
        self.metadata
            .iter()
            .map(|(id, metadata)| (id, &metadata.source))
            .collect()
    }

    /// Focus the graph for a specific project by ID.
    pub fn focus_for(&self, id_or_alias: &Id, with_dependents: bool) -> miette::Result<Self> {
        let project = self.get(id_or_alias)?;
        let graph = self.to_focused_graph(&project, with_dependents);

        // Copy over metadata
        let mut metadata = FxHashMap::default();

        for new_index in graph.node_indices() {
            let project_id = &graph[new_index].id;

            if let Some(old_node) = self.metadata.get(project_id) {
                let mut new_node = old_node.to_owned();
                new_node.index = new_index;

                metadata.insert(project_id.to_owned(), new_node);
            }
        }

        Ok(Self {
            context: self.context.clone(),
            fs_cache: HashMap::new(),
            graph,
            metadata,
            projects: self.projects.clone(),
        })
    }

    fn internal_get(&self, id_or_alias: &str) -> miette::Result<Arc<Project>> {
        let id = self.resolve_id(id_or_alias);

        if let Some(project) = self.read_cache().get(&id) {
            return Ok(Arc::clone(project));
        }

        let mut cache = self.write_cache();

        let expander = ProjectExpander::new(ProjectExpanderContext {
            aliases: self.aliases(),
            workspace_root: &self.context.workspace_root,
        });

        let project = Arc::new(expander.expand(self.get_unexpanded(&id)?)?);

        cache.insert(id.clone(), Arc::clone(&project));

        Ok(project)
    }

    fn internal_search(&self, search: &Path) -> miette::Result<Arc<String>> {
        let cache_key = search.to_path_buf();

        if let Some(cache) = self.fs_cache.read(&cache_key, |_, v| v.clone()) {
            return Ok(cache);
        }

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = String::new();

        for (id, metadata) in &self.metadata {
            if !search.starts_with(metadata.source.as_str()) {
                continue;
            }

            if let Ok(diff) = search.relative_to(metadata.source.as_str()) {
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
            return Err(ProjectGraphError::MissingFromPath(search.to_path_buf()).into());
        }

        let id = Arc::new(possible_id);
        let _ = self.fs_cache.insert(cache_key, Arc::clone(&id));

        Ok(id)
    }

    pub fn resolve_id(&self, id_or_alias: &str) -> Id {
        Id::raw(if self.metadata.contains_key(id_or_alias) {
            id_or_alias
        } else {
            self.metadata
                .iter()
                .find_map(|(id, metadata)| {
                    if metadata
                        .alias
                        .as_ref()
                        .is_some_and(|alias| alias == id_or_alias)
                    {
                        Some(id.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or(id_or_alias)
        })
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

impl GraphData<Project, DependencyScope, Id> for ProjectGraph {
    fn get_graph(&self) -> &DiGraph<Project, DependencyScope> {
        &self.graph
    }

    fn get_node_index(&self, node: &Project) -> NodeIndex {
        self.metadata.get(&node.id).unwrap().index
    }

    fn get_node_key(&self, node: &Project) -> Id {
        node.id.clone()
    }
}

impl GraphConnections<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphConversions<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToDot<Project, DependencyScope, Id> for ProjectGraph {}

impl GraphToJson<Project, DependencyScope, Id> for ProjectGraph {}
