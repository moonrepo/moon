use crate::build_data::*;
use crate::projects_locator::locate_projects_with_globs;
use crate::repo_type::RepoType;
use crate::tasks_querent::*;
use crate::workspace_builder_error::WorkspaceBuilderError;
use crate::workspace_cache::*;
use miette::IntoDiagnostic;
use moon_cache::CacheEngine;
use moon_common::{
    Id, color, consts,
    path::{PathExt, WorkspaceRelativePathBuf, is_root_level_source},
};
use moon_config::{
    ConfigLoader, DependencyConfig, DependencyScope, DependencyType, InheritedTasksManager,
    ProjectsSourcesList, ToolchainConfig, WorkspaceConfig, WorkspaceProjects, finalize_config,
};
use moon_feature_flags::glob_walk_with_options;
use moon_pdk_api::ExtendProjectGraphInput;
use moon_project::Project;
use moon_project_builder::{ProjectBuilder, ProjectBuilderContext};
use moon_project_constraints::{enforce_layer_relationships, enforce_tag_relationships};
use moon_project_graph::{ProjectGraph, ProjectGraphError, ProjectMetadata};
use moon_task::{Target, Task};
use moon_task_builder::TaskDepsBuilder;
use moon_task_graph::{GraphExpanderContext, NodeState, TaskGraph, TaskGraphError, TaskMetadata};
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::BoxedVcs;
use moon_workspace_graph::WorkspaceGraph;
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_events::Emitter;
use starbase_utils::glob::GlobWalkOptions;
use starbase_utils::json;
use std::sync::Arc;
use std::{collections::BTreeMap, path::Path};
use tracing::{debug, instrument, trace};

pub struct WorkspaceBuilderContext<'app> {
    pub config_loader: &'app ConfigLoader,
    pub enabled_toolchains: Vec<Id>,
    pub extend_project: Emitter<ExtendProjectEvent>,
    pub extend_project_graph: Emitter<ExtendProjectGraphEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub toolchain_config: &'app ToolchainConfig,
    pub toolchain_registry: Arc<ToolchainRegistry>,
    pub vcs: Option<Arc<BoxedVcs>>,
    pub working_dir: &'app Path,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilder<'app> {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext<'app>>>,

    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: Vec<WorkspaceRelativePathBuf>,

    /// Aliases to their associated project.
    aliases: FxHashMap<String, Id>,

    /// Projects grouped by tag, for use in task dependency resolution.
    projects_by_tag: FxHashMap<Id, Vec<Id>>,

    /// Mapping of project IDs to associated data required for building
    /// the project itself. Currently we track the following:
    ///   - The alias, derived from manifests (`package.json`).
    ///   - Their `moon.yml` in the project root.
    ///   - Their file source location, relative from the workspace root.
    project_data: FxHashMap<Id, ProjectBuildData>,

    /// The project DAG.
    project_graph: DiGraph<NodeState<Project>, DependencyScope>,

    /// Projects that have explicitly renamed themselves with the `id` setting.
    /// Maps original ID to renamed ID.
    renamed_project_ids: FxHashMap<Id, Id>,

    /// The type of repository: monorepo or polyrepo.
    repo_type: RepoType,

    /// The root project ID (only if a monorepo).
    root_project_id: Option<Id>,

    /// Mapping of task targets to associated data required for building
    /// the project itself. Currently we track the following:
    ///   - Their task options, for resolving deps.
    task_data: FxHashMap<Target, TaskBuildData>,

    /// The task DAG.
    task_graph: DiGraph<NodeState<Task>, DependencyType>,
}

impl<'app> WorkspaceBuilder<'app> {
    #[instrument(skip_all)]
    pub async fn new(
        context: WorkspaceBuilderContext<'app>,
    ) -> miette::Result<WorkspaceBuilder<'app>> {
        debug!("Building workspace graph (project and task graphs)");

        let mut graph = WorkspaceBuilder {
            config_paths: vec![],
            context: Some(Arc::new(context)),
            aliases: FxHashMap::default(),
            projects_by_tag: FxHashMap::default(),
            project_data: FxHashMap::default(),
            project_graph: DiGraph::default(),
            renamed_project_ids: FxHashMap::default(),
            repo_type: RepoType::Unknown,
            root_project_id: None,
            task_data: FxHashMap::default(),
            task_graph: DiGraph::default(),
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
            .expect("VCS is required for workspace graph caching!")
            .is_enabled();
        let mut graph = Self::new(context).await?;

        // No VCS to hash with, so abort caching
        if !is_vcs_enabled {
            graph.load_projects().await?;
            graph.load_tasks().await?;

            return Ok(graph);
        }

        // Hash the project graph based on the preloaded state
        let mut graph_contents = WorkspaceGraphHash::default();
        graph_contents.add_projects(&graph.project_data);
        graph_contents.add_configs(graph.hash_required_configs().await?);
        graph_contents.gather_env();

        let hash = cache_engine
            .hash
            .save_manifest_without_hasher("workspace-graph", &graph_contents)?;

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
        graph.load_tasks().await?;

        state.data.last_hash = hash;
        state.data.projects = graph.project_data.clone();
        state.save()?;

        json::write_file(cache_path, &graph, false)?;

        Ok(graph)
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(mut self) -> miette::Result<WorkspaceGraph> {
        self.enforce_constraints()?;

        let context = self.context.take().unwrap();

        let mut graph_context = GraphExpanderContext {
            working_dir: context.working_dir.to_owned(),
            workspace_root: context.workspace_root.to_owned(),
            ..Default::default()
        };

        // These are only in conditionals for tests that don't have git
        // initialized, which is most of them!
        if let Some(vcs) = &context.vcs {
            if vcs.is_enabled() {
                graph_context.vcs_branch = vcs.get_local_branch().await?;
                graph_context.vcs_revision = vcs.get_local_branch_revision().await?;

                if let Ok(repo) = vcs.get_repository_slug().await {
                    graph_context.vcs_repository = repo;
                }
            } else {
                graph_context.vcs_branch = vcs.get_default_branch().await?;
            }
        }

        let project_metadata = self
            .project_data
            .into_iter()
            .map(|(id, data)| {
                (
                    id,
                    ProjectMetadata {
                        alias: data.alias,
                        index: data.node_index.unwrap_or_default(),
                        original_id: data.original_id,
                        source: data.source,
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        let project_graph = Arc::new(ProjectGraph::new(
            self.project_graph.filter_map(
                |_, node| match node {
                    NodeState::Loading => None,
                    NodeState::Loaded(project) => Some(project.to_owned()),
                },
                |_, edge| Some(*edge),
            ),
            project_metadata,
            graph_context.clone(),
        ));

        let task_metadata = self
            .task_data
            .into_iter()
            .map(|(id, data)| {
                (
                    id,
                    TaskMetadata {
                        index: data.node_index.unwrap_or_default(),
                    },
                )
            })
            .collect::<FxHashMap<_, _>>();

        let task_graph = Arc::new(TaskGraph::new(
            self.task_graph.filter_map(
                |_, node| match node {
                    NodeState::Loading => None,
                    NodeState::Loaded(task) => Some(task.to_owned()),
                },
                |_, edge| Some(*edge),
            ),
            task_metadata,
            graph_context,
            Arc::clone(&project_graph),
        ));

        Ok(WorkspaceGraph::new(project_graph, task_graph))
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
            self.load_project(&id).await?;
        }

        Ok(())
    }

    #[instrument(name = "load_project", skip(self))]
    async fn internal_load_project(
        &mut self,
        id_or_alias: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<(Id, NodeIndex)> {
        let id = ProjectBuildData::resolve_id(id_or_alias, &self.project_data);

        {
            let Some(build_data) = self.project_data.get(&id) else {
                return Err(ProjectGraphError::UnconfiguredID(id).into());
            };

            // Already loaded, exit early with existing index
            if let Some(index) = &build_data.node_index {
                return Ok((id, *index));
            }
        }

        // Not loaded, insert a temporary node so that we have an index
        let index = self.project_graph.add_node(NodeState::Loading);

        self.project_data.get_mut(&id).unwrap().node_index = Some(index);

        // Build the project
        let project = self.build_project(&id).await?;

        cycle.insert(id.clone());

        // Then group projects by relevant data
        for tag in &project.config.tags {
            self.projects_by_tag
                .entry(tag.to_owned())
                .or_default()
                .push(id.clone());
        }

        // Then persist task build data
        for task in project.tasks.values() {
            self.task_data.insert(
                task.target.clone(),
                TaskBuildData {
                    options: task.options.clone(),
                    ..Default::default()
                },
            );
        }

        // Then build dependency projects
        for dep_config in &project.dependencies {
            if cycle.contains(&dep_config.id) {
                debug!(
                    project_id = id.as_str(),
                    dependency_id = dep_config.id.as_str(),
                    "Encountered a dependency cycle (from project); will disconnect nodes to avoid recursion",
                );

                continue;
            }

            let dep = Box::pin(self.internal_load_project(&dep_config.id, cycle)).await?;

            // Don't link the root project to any project, but still load it
            if !dep_config.is_root_scope() {
                self.project_graph.add_edge(index, dep.1, dep_config.scope);
            }
        }

        // And finally, update the node weight state
        *self.project_graph.node_weight_mut(index).unwrap() = NodeState::Loaded(project);

        cycle.clear();

        Ok((id, index))
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
                enabled_toolchains: &context.enabled_toolchains,
                monorepo: self.repo_type.is_monorepo(),
                root_project_id: self.root_project_id.as_ref(),
                toolchain_config: context.toolchain_config,
                toolchain_registry: context.toolchain_registry.clone(),
                workspace_root: context.workspace_root,
            },
        )?;

        if let Some(config) = &build_data.config {
            builder.inherit_local_config(config).await?;
        } else {
            builder.load_local_config().await?;
        }

        builder.inherit_global_config(context.inherited_tasks)?;

        // Inherit from legacy platforms
        let extended_data = context
            .extend_project
            .emit(ExtendProjectEvent {
                project_id: id.to_owned(),
                project_source: build_data.source.to_owned(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?;

        for dep_config in extended_data.dependencies {
            builder.extend_with_dependency(dep_config);
        }

        for (task_id, task_config) in extended_data.tasks {
            builder.extend_with_task(task_id, task_config);
        }

        // Inherit from build data (toolchains, etc)
        for extended_data in &build_data.extensions {
            for dep_config in &extended_data.dependencies {
                builder.extend_with_dependency(DependencyConfig {
                    id: ProjectBuildData::resolve_id(&dep_config.id, &self.project_data),
                    scope: dep_config.scope,
                    ..Default::default()
                });
            }

            for (task_id, task_config) in &extended_data.tasks {
                builder.extend_with_task(Id::new(task_id)?, finalize_config(task_config.clone())?);
            }
        }

        // Inherit alias before building in case the project
        // references itself in tasks or dependencies
        if let Some(alias) = &build_data.alias {
            builder.set_alias(alias);
        }

        let project = builder.build().await?;

        Ok(project)
    }

    /// Load a single task by target into the graph.
    pub async fn load_task(&mut self, target: &Target) -> miette::Result<()> {
        self.internal_load_task(target, &mut FxHashSet::default())
            .await?;

        Ok(())
    }

    /// Load all tasks into the graph, derived from the loaded projects.
    pub async fn load_tasks(&mut self) -> miette::Result<()> {
        let mut targets = vec![];

        for weight in self.project_graph.node_weights() {
            if let NodeState::Loaded(project) = weight {
                for task in project.tasks.values() {
                    targets.push(task.target.clone());
                }
            }
        }

        for target in targets {
            self.load_task(&target).await?;
        }

        Ok(())
    }

    #[instrument(name = "load_task", skip(self))]
    async fn internal_load_task(
        &mut self,
        target: &Target,
        cycle: &mut FxHashSet<Target>,
    ) -> miette::Result<NodeIndex> {
        let target = TaskBuildData::resolve_target(target, &self.project_data)?;

        {
            let Some(build_data) = self.task_data.get(&target) else {
                return Err(TaskGraphError::UnconfiguredTarget(target).into());
            };

            // Already loaded, exit early with existing index
            if let Some(index) = &build_data.node_index {
                return Ok(*index);
            }
        }

        // Not loaded, resolve the task
        let (_, project_index) = self
            .internal_load_project(target.get_project_id()?, &mut FxHashSet::default())
            .await?;

        let NodeState::Loaded(project) = self.project_graph.node_weight_mut(project_index).unwrap()
        else {
            panic!("Unable to load task, owning project is in a non-loaded state!");
        };

        // Not loaded, insert a temporary node so that we have an index
        let index = self.task_graph.add_node(NodeState::Loading);

        self.task_data.get_mut(&target).unwrap().node_index = Some(index);

        // Build the task (remove from project)
        let mut task = project.tasks.remove(&target.task_id).unwrap();

        cycle.insert(target.clone());

        // Resolve the task dependencies so we can link edges correctly
        TaskDepsBuilder {
            querent: Box::new(WorkspaceBuilderTasksQuerent {
                project_data: &self.project_data,
                projects_by_tag: &self.projects_by_tag,
                task_data: &self.task_data,
            }),
            project: Some(project),
            root_project_id: self.root_project_id.as_ref(),
            task: &mut task,
        }
        .build()?;

        // Then resolve dependency tasks
        for dep_config in &task.deps {
            if cycle.contains(&dep_config.target) {
                debug!(
                    task_target = target.as_str(),
                    dependency_target = dep_config.target.as_str(),
                    "Encountered a dependency cycle (from task); will disconnect nodes to avoid recursion",
                );

                continue;
            }

            let dep_index = Box::pin(self.internal_load_task(&dep_config.target, cycle)).await?;

            self.task_graph.add_edge(
                index,
                dep_index,
                if dep_config.optional.is_some_and(|v| v) {
                    DependencyType::Optional
                } else {
                    DependencyType::Required
                },
            );
        }

        // And finally, update the node weight state
        *self.task_graph.node_weight_mut(index).unwrap() = NodeState::Loaded(task);

        cycle.clear();

        Ok(index)
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
        let layer_relationships = context
            .workspace_config
            .constraints
            .enforce_layer_relationships;
        let tag_relationships = &context.workspace_config.constraints.tag_relationships;

        if !layer_relationships && tag_relationships.is_empty() {
            return Ok(());
        }

        let default_scope = DependencyScope::Build;

        for (project_index, project_state) in self.project_graph.node_references() {
            let NodeState::Loaded(project) = project_state else {
                continue;
            };

            let deps: Vec<_> = self
                .project_graph
                .neighbors_directed(project_index, Direction::Outgoing)
                .flat_map(|dep_index| {
                    self.project_graph.node_weight(dep_index).and_then(|dep| {
                        match dep {
                            NodeState::Loading => None,
                            NodeState::Loaded(dep) => {
                                Some((
                                    dep,
                                    // Is this safe?
                                    self.project_graph
                                        .find_edge(project_index, dep_index)
                                        .and_then(|ei| self.project_graph.edge_weight(ei))
                                        .unwrap_or(&default_scope),
                                ))
                            }
                        }
                    })
                })
                .collect();

            for (dep, dep_scope) in deps {
                if layer_relationships {
                    enforce_layer_relationships(project, dep, dep_scope)?;
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
        self.context()
            .vcs
            .as_ref()
            .expect("VCS required!")
            .get_file_hashes(&self.config_paths, true)
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

        // Then extend projects from toolchains
        self.extend_project_build_data().await?;

        // Include all workspace-level config files
        for file in glob_walk_with_options(
            context.workspace_root.join(consts::CONFIG_DIRNAME),
            ["*.{pkl,yml}", "tasks/**/*.{pkl,yml}"],
            GlobWalkOptions::default().cache(),
        )? {
            self.config_paths
                .push(file.relative_to(context.workspace_root).into_diagnostic()?);
        }

        Ok(())
    }

    fn load_project_build_data(&mut self, sources: ProjectsSourcesList) -> miette::Result<()> {
        let context = self.context();
        let config_label = context.config_loader.get_debug_label("moon", false);
        let config_names = context.config_loader.get_project_file_names();
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

            // Hash all project-level config files
            for name in &config_names {
                self.config_paths.push(source.join(name));
            }

            // Load the config file
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
                    debug!(
                        old_id = id.as_str(),
                        new_id = new_id.as_str(),
                        "Project has been configured with an explicit identifier of {}, renaming from {}",
                        color::id(new_id),
                        color::id(id.as_str()),
                    );

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

    async fn extend_project_build_data(&mut self) -> miette::Result<()> {
        let context = self.context();

        debug!("Extending project graph");

        // From platforms
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

        for (project_id, alias) in aliases {
            self.track_alias(project_id, alias)?;
        }

        // From toolchains
        for output in context
            .toolchain_registry
            .extend_project_graph_all(|registry, _| ExtendProjectGraphInput {
                context: registry.create_context(),
                project_sources: self
                    .project_data
                    .iter()
                    .map(|(id, build_data)| (id.to_string(), build_data.source.to_string()))
                    .collect(),
            })
            .await?
        {
            for (project_id, mut project_extend) in output.extended_projects {
                let id = Id::new(project_id)?;

                if !self.project_data.contains_key(&id) {
                    return Err(ProjectGraphError::UnconfiguredID(id).into());
                }

                if let Some(alias) = project_extend.alias.take() {
                    self.track_alias(id.clone(), alias)?;
                }

                if let Some(build_data) = self.project_data.get_mut(&id) {
                    build_data.extensions.push(project_extend);
                }
            }

            for input_file in output.input_files {
                self.config_paths.push(
                    context
                        .toolchain_registry
                        .from_virtual_path(input_file)
                        .relative_to(context.workspace_root)
                        .into_diagnostic()?,
                );
            }
        }

        debug!("Loaded {} project aliases", self.aliases.len());

        Ok(())
    }

    fn track_alias(&mut self, id: Id, alias: String) -> miette::Result<()> {
        // Skip aliases that match its own ID
        if alias == id.as_str() {
            return Ok(());
        }

        // Skip aliases that are an invalid ID format
        if let Err(error) = Id::new(&alias) {
            debug!(
                "Skipping alias {} for project {} as its an invalid format: {error}",
                color::label(&alias),
                color::id(&id),
            );

            return Ok(());
        }

        // Skip aliases that would override an ID
        if self.project_data.contains_key(alias.as_str()) {
            debug!(
                "Skipping alias {} for project {} as it conflicts with the existing project {}",
                color::label(&alias),
                color::id(&id),
                color::id(&alias),
            );

            return Ok(());
        }

        // Skip aliases that collide with another alias
        if let Some(existing_id) = self.aliases.get(&alias) {
            // Skip if the existing ID is already for this ID.
            // This scenario is possible when multiple toolchains
            // extract the same aliases (Bun vs Node, etc).
            if existing_id == &id {
                return Ok(());
            }

            return Err(WorkspaceBuilderError::DuplicateProjectAlias {
                alias: alias.clone(),
                old_id: existing_id.to_owned(),
                new_id: id.clone(),
            }
            .into());
        }

        self.project_data
            .get_mut(&id)
            .expect("Project build data not found!")
            .alias = Some(alias.clone());

        self.aliases.insert(alias, id);

        Ok(())
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext<'app>> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
