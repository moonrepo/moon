use moon_common::Id;
use moon_config::{ProjectsAliasesMap, ProjectsSourcesMap};
use moon_logger::debug;
use moon_project::Project;
use moon_project_builder::ProjectBuilderError;
use moon_query::{Criteria, Queryable};
use moon_utils::{get_workspace_root, path};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_styles::color;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

pub type GraphType = DiGraph<Project, ()>;
pub type IndicesType = FxHashMap<Id, NodeIndex>;

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

    #[serde(skip)]
    query_cache: Arc<RwLock<FxHashMap<String, Vec<Id>>>>,
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
            query_cache: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    /// Return a list of all configured project IDs in ascending order.
    pub fn ids(&self) -> Vec<Id> {
        let mut nodes: Vec<Id> = self.sources.keys().cloned().collect();
        nodes.sort();
        nodes
    }

    /// Return all projects that match the query criteria.
    pub fn query<Q: AsRef<Criteria>>(&self, query: Q) -> miette::Result<Vec<&Project>> {
        let query = query.as_ref();
        let query_input = query.input.as_ref().unwrap();

        {
            if let Some(project_ids) = self.query_cache.read().unwrap().get(query_input) {
                return Ok(project_ids.iter().map(|id| self.get(id).unwrap()).collect());
            }
        }

        debug!(
            target: LOG_TARGET,
            "Filtering projects using query {}",
            color::shell(query_input)
        );

        let mut filtered_projects = vec![];
        let mut project_ids = vec![];

        for project in self.get_all()? {
            if project.matches_criteria(query)? {
                debug!(
                    target: LOG_TARGET,
                    "{} did match the criteria",
                    color::id(&project.id)
                );

                project_ids.push(project.id.clone());
                filtered_projects.push(project);
            } else {
                debug!(
                    target: LOG_TARGET,
                    "{} did {} match the criteria",
                    color::id(&project.id),
                    color::failure("NOT"),
                );
            }
        }

        {
            self.query_cache
                .write()
                .unwrap()
                .insert(query_input.to_owned(), project_ids);
        }

        Ok(filtered_projects)
    }

    /// Return a project with the associated ID. If the project does not
    /// exist or has been misconfigured, return an error.
    pub fn get(&self, alias_or_id: &str) -> miette::Result<&Project> {
        let id = Id::raw(match self.aliases.get(alias_or_id) {
            Some(project_id) => project_id,
            None => alias_or_id,
        });

        let index = self
            .indices
            .get(&id)
            .ok_or_else(|| ProjectBuilderError::UnconfiguredID(id.to_string()))?;

        Ok(self.graph.node_weight(*index).unwrap())
    }

    /// Return all projects from the graph.
    pub fn get_all(&self) -> miette::Result<Vec<&Project>> {
        Ok(self.graph.raw_nodes().iter().map(|n| &n.weight).collect())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    #[track_caller]
    pub fn get_from_path<P: AsRef<Path>>(&self, current_file: P) -> miette::Result<&Project> {
        let current_file = current_file.as_ref();
        let workspace_root = get_workspace_root();

        let file = if current_file == workspace_root {
            PathBuf::from(".")
        } else if let Ok(rel_file) = current_file.strip_prefix(&workspace_root) {
            rel_file.to_path_buf()
        } else {
            current_file.to_path_buf()
        };

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = Id::raw("");

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
            return Err(ProjectBuilderError::MissingFromPath(file).into());
        }

        self.get(&possible_id)
    }

    /// Return a list of direct project IDs that the defined project depends on.
    pub fn get_dependencies_of(&self, project: &Project) -> miette::Result<Vec<Id>> {
        let deps = self
            .graph
            .neighbors_directed(*self.indices.get(&project.id).unwrap(), Direction::Outgoing)
            .map(|idx| self.graph.node_weight(idx).unwrap().id.clone())
            .collect();

        Ok(deps)
    }

    /// Return a list of project IDs that require the defined project.
    pub fn get_dependents_of(&self, project: &Project) -> miette::Result<Vec<Id>> {
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
        graph.map(|_, n| n.id.to_string(), |_, e| *e)
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
