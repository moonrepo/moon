use crate::project_build_data::*;
use crate::projects_locator::locate_projects_with_globs;
use crate::repo_type::RepoType;
use crate::task_build_data::*;
use crate::workspace_builder_error::WorkspaceBuilderError;
use crate::workspace_cache::*;
use moon_cache::CacheEngine;
use moon_common::{
    color, consts,
    path::{is_root_level_source, to_virtual_string, WorkspaceRelativePathBuf},
    Id,
};
use moon_config::{
    ConfigLoader, DependencyScope, InheritedTasksManager, ProjectsSourcesList, ToolchainConfig,
    WorkspaceConfig, WorkspaceProjects,
};
use moon_project::{Project, ProjectError};
use moon_project_builder::{ProjectBuilder, ProjectBuilderContext};
use moon_project_constraints::{enforce_project_type_relationships, enforce_tag_relationships};
use moon_project_graph::{ProjectGraph, ProjectGraphError, ProjectGraphType, ProjectNode};
use moon_task::Target;
use moon_task_graph::{TaskGraph, TaskGraphType, TaskNode};
use moon_vcs::BoxedVcs;
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_events::Emitter;
use starbase_utils::{glob, json};
use std::sync::Arc;
use std::{collections::BTreeMap, path::Path};
use tracing::{debug, instrument, trace};

pub struct WorkspaceBuilderContext<'app> {
    pub config_loader: &'app ConfigLoader,
    pub extend_project: Emitter<ExtendProjectEvent>,
    pub extend_project_graph: Emitter<ExtendProjectGraphEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub strict_project_ids: bool,
    pub toolchain_config: &'app ToolchainConfig,
    pub vcs: Option<Arc<BoxedVcs>>,
    pub working_dir: &'app Path,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

pub struct WorkspaceBuildResult {
    pub project_graph: ProjectGraph,
    pub task_graph: TaskGraph,
}

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilder<'app> {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext<'app>>>,

    /// Mapping of project IDs to associated data required for building
    /// the project itself. Currently we track the following:
    ///   - The alias, derived from manifests (`package.json`).
    ///   - Their `moon.yml` in the project root.
    ///   - Their file source location, relative from the workspace root.
    project_data: FxHashMap<Id, ProjectBuildData>,

    /// The project DAG.
    project_graph: ProjectGraphType,

    /// Projects that have explicitly renamed themselves with the `id` setting.
    /// Maps original ID to renamed ID.
    renamed_project_ids: FxHashMap<Id, Id>,

    /// The type of repository: monorepo or polyrepo.
    repo_type: RepoType,

    /// The root project ID (only if a monorepo).
    root_project_id: Option<Id>,

    /// Mapping of task targets to associated data required for building
    /// the task itself.
    task_data: FxHashMap<Target, TaskBuildData>,

    /// The task DAG.
    task_graph: TaskGraphType,
}

impl<'app> WorkspaceBuilder<'app> {
    #[instrument(skip_all)]
    pub async fn new(
        context: WorkspaceBuilderContext<'app>,
    ) -> miette::Result<WorkspaceBuilder<'app>> {
        debug!("Building workspace graph (project and task graphs)");

        let mut graph = WorkspaceBuilder {
            context: Some(Arc::new(context)),
            project_data: FxHashMap::default(),
            project_graph: ProjectGraphType::default(),
            renamed_project_ids: FxHashMap::default(),
            repo_type: RepoType::Unknown,
            root_project_id: None,
            task_data: FxHashMap::default(),
            task_graph: TaskGraphType::default(),
        };

        graph.preload_build_data().await?;
        graph.determine_repo_type()?;

        Ok(graph)
    }

    #[instrument(skip_all)]
    pub async fn new_with_cache(
        context: WorkspaceBuilderContext<'app>,
        cache_engine: &CacheEngine,
    ) -> miette::Result<WorkspaceBuilder<'app>> {
        let is_vcs_enabled = context
            .vcs
            .as_ref()
            .expect("VCS is required for project graph caching!")
            .is_enabled();
        let mut graph = Self::new(context).await?;

        // No VCS to hash with, so abort caching
        if !is_vcs_enabled {
            graph.load_projects().await?;

            return Ok(graph);
        }

        // Hash the project graph based on the preloaded state
        let mut graph_contents = WorkspaceGraphHash::default();
        graph_contents.add_projects(&graph.project_data);
        graph_contents.add_configs(graph.hash_required_configs().await?);
        graph_contents.gather_env();

        let hash = cache_engine
            .hash
            .save_manifest_without_hasher("Workspace graph", &graph_contents)?;

        debug!(hash, "Generated hash for workspace graph");

        // Check the current state and cache
        let mut state = cache_engine
            .state
            .load_state::<WorkspaceProjectsCacheState>("projectsBuildData.json")?;
        let cache_path = cache_engine.state.resolve_path("workspaceGraph.json");

        if hash == state.data.last_hash && cache_path.exists() {
            debug!(
                cache = ?cache_path,
                "Loading workspace graph with {} projects from cache",
                graph.project_data.len(),
            );

            let mut cache: WorkspaceBuilder = json::read_file(cache_path)?;
            cache.context = graph.context;

            return Ok(cache);
        }

        // Build the graph, update the state, and save the cache
        debug!(
            "Preparing workspace graph with {} projects",
            graph.project_data.len(),
        );

        graph.load_projects().await?;

        state.data.last_hash = hash;
        state.data.projects = graph.project_data.clone();
        state.save()?;

        json::write_file(cache_path, &graph, false)?;

        Ok(graph)
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(mut self) -> miette::Result<WorkspaceBuildResult> {
        self.enforce_constraints()?;

        let context = self.context.take().unwrap();

        let project_nodes = self
            .project_data
            .into_iter()
            .map(|(id, data)| {
                (
                    id,
                    ProjectNode {
                        alias: data.alias,
                        index: data.node_index.unwrap_or_default(),
                        original_id: data.original_id,
                        source: data.source,
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        let mut project_graph =
            ProjectGraph::new(self.project_graph, project_nodes, context.workspace_root);

        project_graph.working_dir = context.working_dir.to_owned();

        let task_nodes = self
            .task_data
            .into_iter()
            .map(|(id, data)| {
                (
                    id,
                    TaskNode {
                        index: data.node_index.unwrap_or_default(),
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        let task_graph = TaskGraph::new(self.task_graph, task_nodes);

        Ok(WorkspaceBuildResult {
            project_graph,
            task_graph,
        })
    }

    /// Load a single project by ID or alias into the graph.
    pub async fn load_project(&mut self, id_or_alias: &str) -> miette::Result<()> {
        self.internal_load_project(id_or_alias, &mut FxHashSet::default())
            .await?;

        Ok(())
    }

    /// Load all projects into the graph, as configured in the workspace.
    pub async fn load_projects(&mut self) -> miette::Result<()> {
        let ids = self.project_data.keys().cloned().collect::<Vec<_>>();

        for id in ids {
            self.internal_load_project(&id, &mut FxHashSet::default())
                .await?;
        }

        Ok(())
    }

    #[instrument(name = "load_project", skip(self, cycle))]
    async fn internal_load_project(
        &mut self,
        id_or_alias: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<(Id, NodeIndex)> {
        let id = self.resolve_project_id(id_or_alias);

        {
            let Some(build_data) = self.project_data.get(&id) else {
                return Err(ProjectGraphError::UnconfiguredID(id).into());
            };

            // Already loaded, exit early with existing index
            if let Some(index) = &build_data.node_index {
                trace!(
                    project_id = id.as_str(),
                    "Project already exists in the project graph, skipping load",
                );

                return Ok((id, *index));
            }
        }

        // Not loaded, build the project
        trace!(
            project_id = id.as_str(),
            "Project does not exist in the project graph, attempting to load",
        );

        let mut project = self.build_project(&id).await?;

        cycle.insert(id.clone());

        // Then build dependency projects
        let mut edges = vec![];

        for dep_config in &mut project.dependencies {
            if cycle.contains(&dep_config.id) {
                debug!(
                    project_id = id.as_str(),
                    dependency_id = dep_config.id.as_str(),
                    "Encountered a dependency cycle (from project); will disconnect nodes to avoid recursion",
                );

                continue;
            }

            let dep = Box::pin(self.internal_load_project(&dep_config.id, cycle)).await?;
            let dep_id = dep.0;

            // Don't link the root project to any project, but still load it
            if !dep_config.is_root_scope() {
                edges.push((dep.1, dep_config.scope));
            }

            // TODO is this needed?
            if dep_id != dep_config.id {
                dep_config.id = dep_id;
            }
        }

        // Create task relationships
        for task in project.tasks.values() {
            let task_index = self.internal_load_task(&task.target)?;

            for dep_config in &task.deps {
                let dep_task_index = self.internal_load_task(&dep_config.target)?;

                self.task_graph
                    .add_edge(task_index, dep_task_index, dep_config.get_type());
            }
        }

        // And finally add to the graph
        let project_index = self.project_graph.add_node(project);

        self.project_data.get_mut(&id).unwrap().node_index = Some(project_index);

        for edge in edges {
            self.project_graph.add_edge(project_index, edge.0, edge.1);
        }

        cycle.clear();

        Ok((id, project_index))
    }

    fn internal_load_task(&mut self, target: &Target) -> miette::Result<NodeIndex> {
        {
            let Some(build_data) = self.task_data.get(target) else {
                return Err(ProjectError::UnknownTask {
                    task_id: target.task_id.clone(),
                    project_id: target
                        .get_project_id()
                        .expect("Missing project scope for task target!")
                        .to_owned(),
                }
                .into());
            };

            // Already loaded, exit early with existing index
            if let Some(index) = &build_data.node_index {
                trace!(
                    target = target.as_str(),
                    "Task already exists in the task graph, skipping load",
                );

                return Ok(*index);
            }
        }

        // Not loaded, build the project
        trace!(
            target = target.as_str(),
            "Task does not exist in the task graph, attempting to load",
        );

        // And finally add to the graph
        let index = self.task_graph.add_node(target.to_owned());

        self.task_data.get_mut(target).unwrap().node_index = Some(index);

        Ok(index)
    }

    /// Create and build the project with the provided ID and source.
    #[instrument(skip(self))]
    async fn build_project(&mut self, id: &Id) -> miette::Result<Project> {
        debug!(
            project_id = id.as_str(),
            "Building project {}",
            color::id(id)
        );

        let context = self.context();
        let build_data = self.project_data.get(id).unwrap();

        if !build_data.source.to_path(context.workspace_root).exists() {
            return Err(WorkspaceBuilderError::MissingProjectAtSource(
                build_data.source.to_string(),
            )
            .into());
        }

        let mut builder = ProjectBuilder::new(
            id,
            &build_data.source,
            ProjectBuilderContext {
                config_loader: context.config_loader,
                monorepo: self.repo_type.is_monorepo(),
                root_project_id: self.root_project_id.as_ref(),
                toolchain_config: context.toolchain_config,
                workspace_root: context.workspace_root,
            },
        )?;

        if let Some(config) = &build_data.config {
            builder.inherit_local_config(config).await?;
        } else {
            builder.load_local_config().await?;
        }

        builder.inherit_global_config(context.inherited_tasks)?;

        let extended_data = context
            .extend_project
            .emit(ExtendProjectEvent {
                project_id: id.to_owned(),
                project_source: build_data.source.to_owned(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?;

        // Inherit implicit dependencies
        for dep_config in extended_data.dependencies {
            builder.extend_with_dependency(dep_config);
        }

        // Inherit inferred tasks
        for (task_id, task_config) in extended_data.tasks {
            builder.extend_with_task(task_id, task_config);
        }

        // Inherit alias before building in case the project
        // references itself in tasks or dependencies
        if let Some(alias) = &build_data.alias {
            builder.set_alias(alias);
        }

        let project = builder.build().await?;

        // Create task build data
        let mut task_data = FxHashMap::default();

        for task in project.tasks.values() {
            task_data.insert(task.target.clone(), TaskBuildData::default());
        }

        self.task_data.extend(task_data);

        Ok(project)
    }

    /// Determine the repository type/structure based on the number of project
    /// sources, and where the point to.
    fn determine_repo_type(&mut self) -> miette::Result<()> {
        let single_project = self.project_data.len() == 1;
        let mut has_root_project = false;
        let mut root_project_id = None;

        for (id, build_data) in &self.project_data {
            if is_root_level_source(&build_data.source) {
                has_root_project = true;
                root_project_id = Some(id.to_owned());
                break;
            }
        }

        self.repo_type = match (single_project, has_root_project) {
            (true, true) => RepoType::Polyrepo,
            (false, true) => RepoType::MonorepoWithRoot,
            (false, false) | (true, false) => RepoType::Monorepo,
        };

        if self.repo_type == RepoType::MonorepoWithRoot {
            self.root_project_id = root_project_id;
        }

        Ok(())
    }

    /// Enforce project constraints and boundaries after all nodes have been inserted.
    #[instrument(skip_all)]
    fn enforce_constraints(&self) -> miette::Result<()> {
        debug!("Enforcing project constraints");

        let context = self.context();
        let type_relationships = context
            .workspace_config
            .constraints
            .enforce_project_type_relationships;
        let tag_relationships = &context.workspace_config.constraints.tag_relationships;

        if !type_relationships && tag_relationships.is_empty() {
            return Ok(());
        }

        let default_scope = DependencyScope::Build;

        for (project_index, project) in self.project_graph.node_references() {
            let deps: Vec<_> = self
                .project_graph
                .neighbors_directed(project_index, Direction::Outgoing)
                .flat_map(|dep_index| {
                    self.project_graph.node_weight(dep_index).map(|dep| {
                        (
                            dep,
                            // Is this safe?
                            self.project_graph
                                .find_edge(project_index, dep_index)
                                .and_then(|ei| self.project_graph.edge_weight(ei))
                                .unwrap_or(&default_scope),
                        )
                    })
                })
                .collect();

            for (dep, dep_scope) in deps {
                if type_relationships {
                    enforce_project_type_relationships(project, dep, dep_scope)?;
                }

                for (source_tag, required_tags) in tag_relationships {
                    enforce_tag_relationships(project, source_tag, dep, required_tags)?;
                }
            }
        }

        Ok(())
    }

    /// When caching the graph, we must hash all project and workspace
    /// config files that are required to invalidate the cache.
    async fn hash_required_configs(
        &self,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let context = self.context();
        let config_names = context.config_loader.get_project_file_names();
        let mut configs = vec![];

        // Hash all project-level config files
        for build_data in self.project_data.values() {
            for name in &config_names {
                configs.push(build_data.source.join(name).to_string());
            }
        }

        // Hash all workspace-level config files
        for file in glob::walk(
            context.workspace_root.join(consts::CONFIG_DIRNAME),
            ["*.pkl", "tasks/**/*.pkl", "*.yml", "tasks/**/*.yml"],
        )? {
            configs.push(to_virtual_string(
                file.strip_prefix(context.workspace_root).unwrap(),
            )?);
        }

        context
            .vcs
            .as_ref()
            .expect("VCS required!")
            .get_file_hashes(&configs, true, 500)
            .await
    }

    /// Preload the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    async fn preload_build_data(&mut self) -> miette::Result<()> {
        let context = self.context();
        let mut globs = vec![];
        let mut sources = vec![];

        // Gather all project sources
        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.push((id.to_owned(), WorkspaceRelativePathBuf::from(source)));
            }
        };

        match &context.workspace_config.projects {
            WorkspaceProjects::Sources(map) => {
                add_sources(map);
            }
            WorkspaceProjects::Globs(list) => {
                globs.extend(list);
            }
            WorkspaceProjects::Both(cfg) => {
                globs.extend(&cfg.globs);
                add_sources(&cfg.sources);
            }
        };

        if !sources.is_empty() {
            debug!(
                sources = ?sources,
                "Using configured project sources",
            );
        }

        if !globs.is_empty() {
            debug!(
                globs = ?globs,
                "Locating projects with globs",
            );

            locate_projects_with_globs(&context, &globs, &mut sources)?;
        }

        // Load projects and configs first
        self.load_project_build_data(sources)?;

        // Then load aliases and extend projects
        self.load_project_aliases().await?;

        Ok(())
    }

    async fn load_project_aliases(&mut self) -> miette::Result<()> {
        let context = self.context();

        debug!("Extending project graph with aliases");

        let aliases = context
            .extend_project_graph
            .emit(ExtendProjectGraphEvent {
                sources: self
                    .project_data
                    .iter()
                    .map(|(id, build_data)| (id.to_owned(), build_data.source.to_owned()))
                    .collect(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?
            .aliases;

        let mut dupe_aliases = FxHashMap::<String, Id>::default();

        for (id, alias) in aliases {
            let id = self.renamed_project_ids.get(&id).unwrap_or(&id);

            // Skip aliases that match its own ID
            if id == &alias {
                continue;
            }

            // Skip aliases that would override an ID
            if self.project_data.contains_key(alias.as_str()) {
                debug!(
                    "Skipping alias {} for project {} as it conflicts with the existing project {}",
                    color::label(&alias),
                    color::id(id),
                    color::id(&alias),
                );

                continue;
            }

            if let Some(existing_id) = dupe_aliases.get(&alias) {
                // Skip if the existing ID is already for this ID.
                // This scenario is possible when multiple platforms
                // extract the same aliases (Bun vs Node, etc).
                if existing_id == id {
                    continue;
                }

                return Err(WorkspaceBuilderError::DuplicateProjectAlias {
                    alias: alias.clone(),
                    old_id: existing_id.to_owned(),
                    new_id: id.clone(),
                }
                .into());
            }

            dupe_aliases.insert(alias.clone(), id.to_owned());

            self.project_data
                .get_mut(id)
                .expect("Project build data not found!")
                .alias = Some(alias);
        }

        Ok(())
    }

    fn load_project_build_data(&mut self, sources: ProjectsSourcesList) -> miette::Result<()> {
        let context = self.context();
        let config_label = context.config_loader.get_debug_label("moon", false);
        let mut project_data: FxHashMap<Id, ProjectBuildData> = FxHashMap::default();
        let mut renamed_ids = FxHashMap::default();
        let mut dupe_original_ids = FxHashSet::default();

        debug!("Loading projects");

        for (mut id, source) in sources {
            trace!(
                project_id = id.as_str(),
                "Attempting to load {} (optional)",
                color::file(source.join(&config_label))
            );

            let config = context
                .config_loader
                .load_project_config_from_source(context.workspace_root, &source)?;

            let mut build_data = ProjectBuildData {
                source,
                ..Default::default()
            };

            // Track ID renames
            if let Some(new_id) = &config.id {
                if new_id != &id {
                    build_data.original_id = Some(id.clone());

                    if renamed_ids.contains_key(&id) {
                        dupe_original_ids.insert(id.clone());
                    } else {
                        renamed_ids.insert(id.clone(), new_id.to_owned());
                    }

                    id = new_id.to_owned();
                }
            }

            // Check for duplicate IDs
            if let Some(existing_data) = project_data.get(&id) {
                if existing_data.source != build_data.source {
                    return Err(WorkspaceBuilderError::DuplicateProjectId {
                        id: id.clone(),
                        old_source: existing_data.source.to_string(),
                        new_source: build_data.source.to_string(),
                    }
                    .into());
                }
            }

            // Otherwise persist the build data
            build_data.config = Some(config);
            project_data.insert(id, build_data);
        }

        if !dupe_original_ids.is_empty() {
            trace!(
                original_ids = ?dupe_original_ids.iter().collect::<Vec<_>>(),
                "Found multiple renamed projects with the same original ID; will ignore these IDs within lookups"
            );

            for dupe_id in dupe_original_ids {
                renamed_ids.remove(&dupe_id);
            }
        }

        debug!("Loaded {} projects", project_data.len());

        self.project_data.extend(project_data);
        self.renamed_project_ids.extend(renamed_ids);

        Ok(())
    }

    fn resolve_project_id(&self, id_or_alias: &str) -> Id {
        let id = if self.project_data.contains_key(id_or_alias) {
            Id::raw(id_or_alias)
        } else {
            match self.project_data.iter().find_map(|(id, build_data)| {
                if build_data
                    .alias
                    .as_ref()
                    .is_some_and(|alias| alias == id_or_alias)
                {
                    Some(id)
                } else {
                    None
                }
            }) {
                Some(project_id) => project_id.to_owned(),
                None => Id::raw(id_or_alias),
            }
        };

        if self
            .context
            .as_ref()
            .is_some_and(|ctx| ctx.strict_project_ids)
        {
            return id;
        }

        match self.renamed_project_ids.get(&id) {
            Some(new_id) => new_id.to_owned(),
            None => id,
        }
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext<'app>> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
