use moon_cache::CacheEngine;
use moon_config::{
    GlobalProjectConfig, ProjectID, ProjectsAliasesMap, ProjectsSourcesMap, WorkspaceConfig,
    WorkspaceProjects,
};
use moon_error::MoonError;
use moon_logger::{color, debug, map_list, trace};
use moon_platform::{Platform, Platformable, RegisteredPlatforms};
use moon_project::{
    detect_projects_with_globs, Project, ProjectDependency, ProjectDependencySource, ProjectError,
};
use moon_task::{Target, Task};
use moon_utils::path;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, RwLockWriteGuard};

type GraphType = DiGraph<Project, ()>;
type IndicesType = HashMap<ProjectID, NodeIndex>;

const LOG_TARGET: &str = "moon:project-graph";
const READ_ERROR: &str = "Failed to acquire a read lock";
const WRITE_ERROR: &str = "Failed to acquire a write lock";
const ROOT_NODE_ID: &str = "(workspace)";

async fn load_projects_from_cache(
    workspace_root: &Path,
    workspace_config: &WorkspaceConfig,
    engine: &CacheEngine,
) -> Result<ProjectsSourcesMap, ProjectError> {
    let projects = match &workspace_config.projects {
        WorkspaceProjects::Map(map) => map.clone(),
        WorkspaceProjects::List(globs) => {
            let mut cache = engine.cache_projects_state().await?;

            // Return the values from the cache
            if !cache.item.projects.is_empty() {
                debug!(target: LOG_TARGET, "Loading projects from cache");

                return Ok(cache.item.projects);
            }

            // Generate a new projects map by globbing the filesystem
            debug!(
                target: LOG_TARGET,
                "Finding projects with globs: {}",
                map_list(globs, |g| color::file(g))
            );

            let mut map = HashMap::new();

            detect_projects_with_globs(workspace_root, globs, &mut map)?;

            // Update the cache
            cache.item.globs = globs.clone();
            cache.item.projects = map.clone();
            cache.save().await?;

            map
        }
    };

    debug!(
        target: LOG_TARGET,
        "Creating project graph with {} projects",
        projects.len(),
    );

    Ok(projects)
}

pub struct ProjectGraph {
    /// A mapping of an alias to a project ID.
    pub aliases_map: ProjectsAliasesMap,

    /// The global project configuration that all projects inherit from.
    /// Is loaded from `.moon/project.yml`.
    global_config: GlobalProjectConfig,

    /// Projects that have been loaded into scope represented as a DAG.
    graph: Arc<RwLock<GraphType>>,

    /// Mapping of project IDs to node indices, as we need a way
    /// to query the graph by ID as it only supports it by index.
    indices: Arc<RwLock<IndicesType>>,

    /// List of platforms that provide unique functionality.
    pub platforms: RegisteredPlatforms,

    /// The mapping of projects by ID to a relative file system location.
    /// Is the `projects` setting in `.moon/workspace.yml`.
    pub projects_map: ProjectsSourcesMap,

    /// The workspace configuration. Necessary for project variants.
    /// Is loaded from `.moon/workspace.yml`.
    pub workspace_config: WorkspaceConfig,

    /// The workspace root, in which projects are relatively loaded from.
    pub workspace_root: PathBuf,
}

impl Platformable for ProjectGraph {
    fn register_platform(&mut self, platform: Box<dyn Platform>) -> Result<(), MoonError> {
        let mut platform = platform;

        platform.load_project_graph_aliases(
            &self.workspace_root,
            &self.workspace_config,
            &self.projects_map,
            &mut self.aliases_map,
        )?;

        self.platforms.push(platform);

        Ok(())
    }
}

impl ProjectGraph {
    pub async fn generate(
        workspace_root: &Path,
        workspace_config: &WorkspaceConfig,
        global_config: GlobalProjectConfig,
        cache: &CacheEngine,
    ) -> Result<ProjectGraph, ProjectError> {
        let mut graph = DiGraph::new();

        // Add a virtual root node
        graph.add_node(Project {
            id: ROOT_NODE_ID.to_owned(),
            root: workspace_root.to_path_buf(),
            source: String::from("."),
            ..Project::default()
        });

        Ok(ProjectGraph {
            aliases_map: HashMap::new(),
            global_config,
            graph: Arc::new(RwLock::new(graph)),
            indices: Arc::new(RwLock::new(HashMap::new())),
            platforms: vec![],
            projects_map: load_projects_from_cache(workspace_root, workspace_config, cache).await?,
            workspace_config: workspace_config.clone(),
            workspace_root: workspace_root.to_path_buf(),
        })
    }

    /// Return a list of all configured project IDs in ascending order.
    pub fn ids(&self) -> Vec<ProjectID> {
        let mut nodes: Vec<ProjectID> = self.projects_map.keys().cloned().collect();
        nodes.sort();
        nodes
    }

    /// Return a project with the associated ID. If the project
    /// has not been loaded, it will be loaded and inserted into the
    /// project graph. If the project does not exist or has been
    /// misconfigured, an error will be returned.
    #[track_caller]
    pub fn load(&self, alias_or_id: &str) -> Result<Project, ProjectError> {
        let id = self.resolve_id(alias_or_id);

        // Check if the project already exists in read-only mode,
        // so that it may be dropped immediately after!
        {
            let indices = self.indices.read().expect(READ_ERROR);

            if let Some(index) = indices.get(&id) {
                let graph = self.graph.read().expect(READ_ERROR);

                return Ok(graph.node_weight(*index).unwrap().clone());
            }
        }

        // Otherwise we need to load the project in write mode
        let mut indices = self.indices.write().expect(WRITE_ERROR);
        let mut graph = self.graph.write().expect(WRITE_ERROR);
        let index = self.internal_load(&id, &mut indices, &mut graph)?;

        Ok(graph.node_weight(index).unwrap().clone())
    }

    /// Force load all projects into the graph. This is necessary
    /// when needing to access project *dependents*, and may also
    /// be a costly operation!
    #[track_caller]
    pub fn load_all(&self) -> Result<(), ProjectError> {
        let mut indices = self.indices.write().expect(WRITE_ERROR);
        let mut graph = self.graph.write().expect(WRITE_ERROR);

        for id in self.ids() {
            self.internal_load(&id, &mut indices, &mut graph)?;
        }

        Ok(())
    }

    /// Find and return a project based on the initial path location.
    /// This will attempt to find the closest matching project source.
    #[track_caller]
    pub fn load_from_path<P: AsRef<Path>>(&self, current_file: P) -> Result<Project, ProjectError> {
        let current_file = current_file.as_ref();

        let file = if current_file == self.workspace_root {
            PathBuf::from(".")
        } else if current_file.starts_with(&self.workspace_root) {
            current_file
                .strip_prefix(&self.workspace_root)
                .unwrap()
                .to_path_buf()
        } else {
            current_file.to_path_buf()
        };

        // Find the deepest matching path in case sub-projects are being used
        let mut remaining_length = 1000; // Start with a really fake number
        let mut possible_id = String::new();

        for (id, source) in &self.projects_map {
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

        self.load(&possible_id)
    }

    /// Return a list of direct project IDs that the defined project depends on.
    #[track_caller]
    pub fn get_dependencies_of(&self, project: &Project) -> Result<Vec<ProjectID>, ProjectError> {
        let indices = self.indices.read().expect(READ_ERROR);
        let graph = self.graph.read().expect(READ_ERROR);

        let deps = graph
            .neighbors_directed(*indices.get(&project.id).unwrap(), Direction::Outgoing)
            .map(|idx| graph.node_weight(idx).unwrap().id.clone())
            .collect();

        Ok(deps)
    }

    /// Return a list of project IDs that require the defined project.
    #[track_caller]
    pub fn get_dependents_of(&self, project: &Project) -> Result<Vec<ProjectID>, ProjectError> {
        let indices = self.indices.read().expect(READ_ERROR);
        let graph = self.graph.read().expect(READ_ERROR);

        let deps = graph
            .neighbors_directed(*indices.get(&project.id).unwrap(), Direction::Incoming)
            .map(|idx| graph.node_weight(idx).unwrap().id.clone())
            .filter(|id| id != ROOT_NODE_ID)
            .collect();

        Ok(deps)
    }

    /// Resolve a project ID from the provided value, which can be an ID or alias.
    pub fn resolve_id(&self, alias_or_id: &str) -> String {
        match self.aliases_map.get(alias_or_id) {
            Some(project_id) => project_id.to_owned(),
            None => alias_or_id.to_owned(),
        }
    }

    /// Format as a DOT string.
    pub fn to_dot(&self) -> String {
        let graph = self.graph.read().expect(READ_ERROR);
        let labeled_graph = graph.map(|_, n| n.id.clone(), |_, e| e);
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

                if id == ROOT_NODE_ID {
                    format!(
                        "label=\"{}\" style=filled, shape=oval, fillcolor=black, fontcolor=white",
                        id
                    )
                // } else if id == &highlight_id {
                //     String::from("style=filled, shape=circle, fillcolor=palegreen, fontcolor=black")
                } else {
                    format!(
                        "label=\"{}\" style=filled, shape=oval, fillcolor=gray, fontcolor=black",
                        id
                    )
                }
            },
        );

        format!("{:?}", dot)
    }

    fn create_project(&self, id: &str, source: &str) -> Result<Project, ProjectError> {
        let mut project = Project::new(id, source, &self.workspace_root, &self.global_config)?;
        project.alias = self.find_alias_for_id(id);

        for platform in &self.platforms {
            if !platform.matches(&project.config, None) {
                continue;
            }

            // Determine implicit dependencies
            for dep_cfg in platform.load_project_implicit_dependencies(
                id,
                &project.root,
                &project.config,
                &self.aliases_map,
            )? {
                // Implicit deps should not override explicit deps
                project
                    .dependencies
                    .entry(dep_cfg.id.clone())
                    .or_insert_with(|| {
                        let mut dep = ProjectDependency::from_config(&dep_cfg);
                        dep.source = ProjectDependencySource::Implicit;
                        dep
                    });
            }

            // Inherit platform specific tasks
            for (task_id, task_config) in platform.load_project_tasks(
                id,
                &project.root,
                &project.config,
                &self.workspace_root,
                &self.workspace_config,
            )? {
                // Inferred tasks should not override explicit tasks
                #[allow(clippy::map_entry)]
                if !project.tasks.contains_key(&task_id) {
                    let task = Task::from_config(Target::new(id, &task_id)?, &task_config)?;

                    project.tasks.insert(task_id, task);
                }
            }
        }

        // Expand all tasks for the project (this must happen last)
        project.expand_tasks(
            &self.workspace_root,
            &self.workspace_config.runner.implicit_deps,
            &self.workspace_config.runner.implicit_inputs,
        )?;

        Ok(project)
    }

    /// Find the alias for a given ID. This is currently... not performant,
    /// so revisit once it becomes an issue!
    fn find_alias_for_id(&self, id: &str) -> Option<String> {
        for (alias, project_id) in &self.aliases_map {
            if project_id == id {
                return Some(alias.clone());
            }
        }

        None
    }

    /// Internal method for lazily loading a project and its
    /// dependencies into the graph.
    fn internal_load(
        &self,
        alias_or_id: &str,
        indices: &mut RwLockWriteGuard<IndicesType>,
        graph: &mut RwLockWriteGuard<GraphType>,
    ) -> Result<NodeIndex, ProjectError> {
        let id = self.resolve_id(alias_or_id);

        // Already loaded, abort early
        if indices.contains_key(&id) || id == ROOT_NODE_ID {
            trace!(
                target: LOG_TARGET,
                "Project {} already exists in the project graph",
                color::id(&id),
            );

            return Ok(*indices.get(&id).unwrap());
        }

        trace!(
            target: LOG_TARGET,
            "Project {} does not exist in the project graph, attempting to load",
            color::id(&id),
        );

        // Create project based on ID and source
        let source = match self.projects_map.get(&id) {
            Some(path) => path,
            None => return Err(ProjectError::UnconfiguredID(id)),
        };

        let project = self.create_project(&id, source)?;
        let depends_on = project.get_dependency_ids();

        // Insert the project into the graph
        let node_index = graph.add_node(project);
        graph.add_edge(NodeIndex::new(0), node_index, ());
        indices.insert(id.to_owned(), node_index);

        if !depends_on.is_empty() {
            trace!(
                target: LOG_TARGET,
                "Adding dependencies {} to project {}",
                map_list(&depends_on, |d| color::symbol(d)),
                color::id(id),
            );

            for dep_id in depends_on {
                let dep_index = self.internal_load(dep_id.as_str(), indices, graph)?;
                graph.add_edge(node_index, dep_index, ());
            }
        }

        Ok(node_index)
    }
}
