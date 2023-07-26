use crate::project_graph_error::ProjectGraphError;
use miette::IntoDiagnostic;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::{color, Id};
use moon_config::DependencyScope;
use moon_project::Project;
use moon_query::{build_query, Criteria, Queryable};
use moon_task_expander::TasksExpander;
use once_map::OnceMap;
use pathdiff::diff_paths;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::Serialize;
use starbase_utils::json;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, trace};

pub type GraphType = DiGraph<Project, DependencyScope>;
pub type ProjectsCache = FxHashMap<Id, Arc<Project>>;

#[derive(Serialize)]
pub struct ProjectGraphCache<'graph> {
    graph: &'graph GraphType,
    projects: &'graph ProjectsCache,
}

#[derive(Default)]
pub struct ProjectNode {
    pub alias: Option<String>,
    pub index: NodeIndex,
    pub source: WorkspaceRelativePathBuf,
}

#[derive(Default)]
pub struct ProjectGraph {
    /// Directed-acyclic graph (DAG) of non-expanded projects and their dependencies.
    graph: GraphType,

    /// Graph node information, mapped by project ID.
    pub nodes: FxHashMap<Id, ProjectNode>,

    /// Expanded projects, mapped by project ID.
    projects: Arc<RwLock<ProjectsCache>>,

    /// Cache of query results, mapped by query input to project IDs.
    query_cache: OnceMap<String, Vec<Id>>,

    /// Workspace root, required for expansion.
    workspace_root: PathBuf,
}

impl ProjectGraph {
    pub fn new(graph: GraphType, nodes: FxHashMap<Id, ProjectNode>, workspace_root: &Path) -> Self {
        debug!("Creating project graph");

        Self {
            graph,
            nodes,
            projects: Arc::new(RwLock::new(FxHashMap::default())),
            workspace_root: workspace_root.to_owned(),
            query_cache: OnceMap::new(),
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
        let id = self.resolve_id(alias_or_id);

        // Check if the expanded project has been created, if so return it
        {
            if let Some(project) = self.read_cache().get(&id) {
                return Ok(Arc::clone(project));
            }
        }

        // Otherwise expand the project and cache it with an Arc
        {
            let project = self.expand_project(&id)?;

            self.write_cache()
                .insert(project.id.clone(), Arc::new(project));
        }

        Ok(Arc::clone(self.read_cache().get(&id).unwrap()))
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

    /// Return all expanded projects that match the query criteria.
    pub fn query<Q: AsRef<Criteria>>(&self, query: Q) -> miette::Result<Vec<Arc<Project>>> {
        let mut projects = vec![];

        for id in self.raw_query(query)? {
            projects.push(self.get(id)?);
        }

        Ok(projects)
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

    /// Format graph as a JSON string.
    pub fn to_json(&self) -> miette::Result<String> {
        let projects = self.read_cache();

        json::to_string_pretty(&ProjectGraphCache {
            graph: &self.graph,
            projects: &projects,
        })
        .into_diagnostic()
    }

    fn expand_project(&self, id: &Id) -> miette::Result<Project> {
        debug!(id = id.as_str(), "Expanding project {}", color::id(id));

        let unexpanded_project = self.get_unexpanded(id)?;
        let mut project = unexpanded_project.to_owned();
        let mut expander = TasksExpander {
            project: &mut project,
            workspace_root: &self.workspace_root,
        };

        let query = |input: String| {
            let mut results = vec![];

            // Don't get() the expanded projects, since it'll overflow the
            // stack trying to recursively expand projects! Using unexpanded
            // dependent projects works just fine for the expansion process.
            for id in self.raw_query(build_query(input)?)? {
                results.push(self.get_unexpanded(id)?);
            }

            Ok(results)
        };

        for task_id in unexpanded_project.tasks.keys() {
            // Resolve in this order!
            expander.expand_env(task_id)?;
            expander.expand_deps(task_id, query)?;
            expander.expand_inputs(task_id)?;
            expander.expand_outputs(task_id)?;
            expander.expand_args(task_id)?;
            expander.expand_command(task_id)?;
        }

        Ok(project)
    }

    fn get_unexpanded(&self, id: &Id) -> miette::Result<&Project> {
        let node = self
            .nodes
            .get(id)
            .ok_or_else(|| ProjectGraphError::UnconfiguredID(id.to_owned()))?;

        Ok(self.graph.node_weight(node.index).unwrap())
    }

    fn raw_query<Q: AsRef<Criteria>>(&self, query: Q) -> miette::Result<&[Id]> {
        let query = query.as_ref();
        let query_input = query
            .input
            .clone()
            .expect("Querying the project graph requires a query input string.");

        self.query_cache.try_insert(query_input.clone(), |_| {
            debug!("Querying projects with {}", color::shell(query_input));

            let mut project_ids = vec![];

            // Don't use `get_all` as it recursively calls `query`,
            // which runs into a deadlock! This should be faster also...
            for node in self.graph.raw_nodes() {
                let project = &node.weight;

                if project.matches_criteria(query)? {
                    debug!("{} did match the criteria", color::id(&project.id));

                    project_ids.push(project.id.clone());
                } else {
                    trace!(
                        "{} did {} match the criteria",
                        color::id(&project.id),
                        color::failure("NOT"),
                    );
                }
            }

            Ok(project_ids)
        })
    }

    fn resolve_id(&self, alias_or_id: &str) -> Id {
        Id::raw(if self.nodes.contains_key(alias_or_id) {
            alias_or_id
        } else {
            self.nodes
                .iter()
                .find(|(_, node)| node.alias.as_ref().is_some_and(|a| a == alias_or_id))
                .map(|(id, _)| id.as_str())
                .unwrap_or(alias_or_id)
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
