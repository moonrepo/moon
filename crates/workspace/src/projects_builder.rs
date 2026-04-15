use crate::projects_locator::locate_projects_with_globs;
use crate::repo_type::RepoType;
use crate::tasks_querent::WorkspaceTasksQuerent;
use crate::workspace_builder::WorkspaceBuilderContext;
use crate::workspace_builder_error::WorkspaceBuilderError;
use daggy::{Dag, NodeIndex};
use miette::IntoDiagnostic;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, is_root_level_source};
use moon_common::{Id, color};
use moon_config::{
    DependencyScope, ProjectConfig, ProjectDependencyConfig, WorkspaceProjectGlobFormat,
    WorkspaceProjects, finalize_config,
};
use moon_graph_utils::{GraphExpanderContext, NodeState};
use moon_pdk_api::{ExtendProjectGraphInput, ExtendProjectGraphOutput, ExtendProjectOutput};
use moon_project::{Project, ProjectAlias};
use moon_project_builder::{ProjectBuilder, ProjectBuilderContext};
use moon_project_constraints::{enforce_layer_relationships, enforce_tag_relationships};
use moon_project_graph::{ProjectGraph, ProjectGraphError, ProjectNode};
use moon_task::{Target, Task, TaskOptions};
use moon_task_builder::TaskDepsBuilder;
use petgraph::prelude::*;
use petgraph::visit::IntoNodeReferences;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::mem;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument, trace};

pub type ProjectDag = Dag<NodeState<Project>, DependencyScope>;
pub type ProjectBuildDataMap = FxHashMap<Id, ProjectBuildData>;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ProjectBuildData {
    /// Map of aliases to the plugin that provided them.
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    pub aliases: FxHashMap<String, Id>,

    #[serde(skip)]
    pub config: Option<ProjectConfig>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<ExtendProjectOutput>,

    // Only used for renaming!
    #[serde(skip)]
    pub id: Option<Id>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    pub source: WorkspaceRelativePathBuf,
}

impl ProjectBuildData {
    pub fn rename_id_if_configured(&mut self) -> Option<(Id, Id)> {
        if let Some(old_id) = &self.id
            && let Some(config) = &self.config
            && let Some(new_id) = &config.id
            && new_id != old_id
        {
            return Some((old_id.to_owned(), new_id.to_owned()));
        }

        None
    }

    // TODO deprecated
    pub fn resolve_id(id_or_alias: &str, project_data: &ProjectBuildDataMap) -> Id {
        if project_data.contains_key(id_or_alias) {
            Id::raw(id_or_alias)
        } else {
            match project_data.iter().find_map(|(id, build_data)| {
                if build_data.aliases.contains_key(id_or_alias) {
                    Some(id)
                } else {
                    None
                }
            }) {
                Some(project_id) => project_id.to_owned(),
                None => Id::raw(id_or_alias),
            }
        }
    }
}

pub fn load_project_build_data(
    context: Arc<WorkspaceBuilderContext>,
    id: Id,
    source: WorkspaceRelativePathBuf,
) -> miette::Result<ProjectBuildData> {
    let config = context
        .config_loader
        .load_project_config_from_source(&context.workspace_root, &source)?;

    Ok(ProjectBuildData {
        config: Some(config),
        id: Some(id),
        source,
        ..Default::default()
    })
}

pub async fn extend_project_build_data_with_plugins(
    context: Arc<WorkspaceBuilderContext>,
    sources: BTreeMap<Id, String>,
) -> miette::Result<Vec<(Id, ExtendProjectGraphOutput, bool)>> {
    let mut outputs = vec![];

    // From toolchains
    for result in context
        .toolchain_registry
        .extend_project_graph_all(|registry, toolchain| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: sources.clone(),
            toolchain_config: registry.create_config(&toolchain.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, true));
    }

    // From extensions
    for result in context
        .extension_registry
        .extend_project_graph_all(|registry, extension| ExtendProjectGraphInput {
            context: registry.create_context(),
            project_sources: sources.clone(),
            extension_config: registry.create_config(&extension.id),
            ..Default::default()
        })
        .await?
    {
        outputs.push((result.id, result.output, false));
    }

    Ok(outputs)
}

pub async fn build_project(
    context: Arc<WorkspaceBuilderContext>,
    build_data: ProjectBuildData,
    id: Id,
    root_id: Option<Id>,
    monorepo: bool,
) -> miette::Result<Project> {
    if !build_data.source.to_path(&context.workspace_root).exists() {
        return Err(
            WorkspaceBuilderError::MissingProjectAtSource(build_data.source.to_string()).into(),
        );
    }

    let mut builder = ProjectBuilder::new(
        &id,
        &build_data.source,
        ProjectBuilderContext {
            config_loader: &context.config_loader,
            enabled_toolchains: &context.enabled_toolchains,
            monorepo,
            root_project_id: root_id.as_ref(),
            toolchains_config: &context.toolchains_config,
            toolchain_registry: context.toolchain_registry.clone(),
            workspace_root: &context.workspace_root,
        },
    )?;

    // Inherit configs and tasks
    if let Some(config) = build_data.config {
        builder.inherit_local_config(&config).await?;
    } else {
        builder.load_local_config().await?;
    }

    builder.inherit_global_configs(&context.inherited_tasks)?;

    // Inherit from build data and plugins (toolchains, etc)
    for extended_data in build_data.extensions {
        for dep_config in extended_data.dependencies {
            builder.extend_with_dependency(ProjectDependencyConfig {
                id: dep_config.id,
                scope: dep_config.scope,
                ..Default::default()
            });
        }

        for (task_id, task_config) in extended_data.tasks {
            builder.extend_with_task(task_id, finalize_config(task_config)?);
        }
    }

    // Inherit aliases before building in case the project
    // references itself in tasks or dependencies
    builder.set_aliases(
        build_data
            .aliases
            .into_iter()
            .map(|(alias, plugin)| ProjectAlias { alias, plugin })
            .collect(),
    );

    builder.build().await
}

#[derive(Deserialize, Serialize)]
pub struct WorkspaceProjectsBuilder {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// Map of aliases to project IDs.
    aliases_to_ids: FxHashMap<String, Id>,

    /// Cached projects build data.
    build_data: ProjectBuildDataMap,

    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: FxHashSet<WorkspaceRelativePathBuf>,

    /// Map of project IDs to their graph index.
    pub ids_to_indexes: FxHashMap<Id, NodeIndex>,

    /// Map of project IDs to task options, indexed by target.
    ids_to_target_options: FxHashMap<Id, FxHashMap<Target, TaskOptions>>,

    /// The project DAG.
    pub graph: ProjectDag,

    /// Map of original project IDs to renamed IDs.
    renamed_ids: FxHashMap<Id, Id>,

    /// The type of repository: monorepo or polyrepo.
    repo_type: RepoType,

    /// The root project ID (only if a monorepo).
    root_id: Option<Id>,

    /// Map of tag IDs to a list of project IDs that belong to the tag.
    tags_to_ids: FxHashMap<Id, Vec<Id>>,
}

impl WorkspaceProjectsBuilder {
    pub fn get_or_insert_node(&mut self, id_or_alias: &str) -> NodeIndex {
        // Edge may be linking with an alias, but we should use the ID
        let id = self.resolve_id(id_or_alias);

        match self.ids_to_indexes.get(&id) {
            Some(index) => *index,
            None => {
                let index = self.graph.add_node(NodeState::Loading);
                self.ids_to_indexes.insert(id, index);
                index
            }
        }
    }

    pub fn insert_or_update_node(&mut self, project: Project) {
        // Project node may have been inserted through an edge first,
        // so we need to update the state from loading to loaded
        if let Some(index) = self.ids_to_indexes.get(&project.id)
            && let Some(node) = self.graph.node_weight_mut(*index)
        {
            *node = NodeState::Loaded(project);
        }
        // Otherwise the node was inserted first, so we can set as loaded
        else {
            self.ids_to_indexes.insert(
                project.id.clone(),
                self.graph.add_node(NodeState::Loaded(project)),
            );
        }
    }

    pub fn resolve_id(&self, id_or_alias: &str) -> Id {
        if let Some(id) = self.aliases_to_ids.get(id_or_alias) {
            id.to_owned()
        } else {
            Id::raw(id_or_alias)
        }
    }
}

impl WorkspaceProjectsBuilder {
    pub fn new(context: Arc<WorkspaceBuilderContext>) -> Self {
        Self {
            context: Some(context),
            aliases_to_ids: FxHashMap::default(),
            build_data: ProjectBuildDataMap::default(),
            config_paths: FxHashSet::default(),
            ids_to_indexes: FxHashMap::default(),
            ids_to_target_options: FxHashMap::default(),
            graph: ProjectDag::default(),
            renamed_ids: FxHashMap::default(),
            repo_type: RepoType::Unknown,
            root_id: None,
            tags_to_ids: FxHashMap::default(),
        }
    }

    /// Preload projects and their configs for use within caching.
    #[instrument(skip(self))]
    pub async fn preload(&mut self) -> miette::Result<()> {
        self.build_data = self.load().await?;

        Ok(())
    }

    /// Load and build all projects into the graph, as configured in the workspace.
    #[instrument(skip(self))]
    pub async fn build(&mut self, ids: Option<Vec<Id>>) -> miette::Result<()> {
        let data = if self.build_data.is_empty() {
            self.load().await?
        } else {
            mem::take(&mut self.build_data)
        };

        self.determine_repo_type(&data)?;
        self.build_graph(ids, data).await?;
        self.enforce_constraints()?;

        Ok(())
    }

    // Extract all tasks from their respective project, as the data will live
    // in the task graph and not the project graph!
    #[instrument(skip(self))]
    pub fn extract_tasks(&mut self) -> miette::Result<Vec<Task>> {
        let mut tasks = vec![];

        for project_state in self.graph.node_weights_mut() {
            let NodeState::Loaded(project) = project_state else {
                continue;
            };

            for (_, mut task) in mem::take(&mut project.tasks) {
                // Resolve the task dependencies so we can link edges
                // correctly when building the task graph
                if !task.deps.is_empty() {
                    TaskDepsBuilder {
                        querent: Box::new(WorkspaceTasksQuerent {
                            aliases_to_ids: &self.aliases_to_ids,
                            ids_to_target_options: &self.ids_to_target_options,
                            tags_to_ids: &self.tags_to_ids,
                        }),
                        project: Some(project),
                        root_project_id: self.root_id.as_ref(),
                        task: &mut task,
                    }
                    .build()?;
                }

                tasks.push(task);
            }
        }

        // Free up some memory
        mem::take(&mut self.ids_to_target_options);
        mem::take(&mut self.tags_to_ids);

        Ok(tasks)
    }

    pub fn finalize(self, context: GraphExpanderContext) -> ProjectGraph {
        let mut project_graph = ProjectGraph::new(context);
        project_graph.default_id = self.context().workspace_config.default_project.clone();
        project_graph.aliases.extend(self.aliases_to_ids);
        let mut loaded_projects = FxHashMap::default();

        // TODO switch to filter_map_owned
        project_graph.graph = self.graph.filter_map(
            |ni, node| match node {
                NodeState::Loading => None,
                NodeState::Loaded(project) => {
                    loaded_projects.insert(ni, project.to_owned());

                    Some(ni)
                }
            },
            |_, edge| Some(*edge),
        );

        for index in project_graph.graph.graph().node_indices() {
            let old_index = *project_graph.graph.node_weight(index).unwrap();
            let project = loaded_projects.remove(&old_index).unwrap();
            let id = project.id.clone();

            project_graph.indexes.insert(index, id.clone());
            project_graph
                .nodes
                .insert(id, ProjectNode { index, project });
        }

        project_graph
    }

    #[instrument(skip(self))]
    async fn build_graph(
        &mut self,
        ids: Option<Vec<Id>>,
        projects_data: ProjectBuildDataMap,
    ) -> miette::Result<()> {
        let concurrency = num_cpus::get();
        let mut set = JoinSet::new();
        let mut queue = projects_data
            .into_iter()
            .filter(|(id, _)| ids.as_ref().is_none_or(|list| list.contains(id)))
            .collect::<VecDeque<_>>();

        loop {
            // Build each project in the background
            if let Some((id, build_data)) = queue.pop_front() {
                debug!(
                    project_id = id.as_str(),
                    "Building project {}",
                    color::id(&id)
                );

                set.spawn(Box::pin(build_project(
                    self.context(),
                    build_data,
                    id,
                    self.root_id.clone(),
                    self.repo_type.is_monorepo(),
                )));
            }

            // Keep enqueuing projects until we hit the concurrency limit
            if set.len() < concurrency && !queue.is_empty() {
                continue;
            }

            // If the queue is empty, break the loop
            let Some(result) = set.join_next().await else {
                break;
            };

            let mut project = result.into_diagnostic()??;
            let from_index = self.get_or_insert_node(&project.id);

            // Resolve dependency IDs as they may be aliases
            for dep_config in &mut project.dependencies {
                dep_config.id = self.resolve_id(&dep_config.id);

                // And then create edges but don't link the root project
                if !dep_config.is_root_scope() {
                    let to_index = self.get_or_insert_node(&dep_config.id);

                    self.graph
                        .add_edge(from_index, to_index, dep_config.scope)
                        .map_err(|_| ProjectGraphError::WouldCycle {
                            source_id: project.id.to_string(),
                            target_id: dep_config.id.to_string(),
                        })?;
                }
            }

            // Extract tags and group projects
            for tag in &project.config.tags {
                self.tags_to_ids
                    .entry(tag.to_owned())
                    .or_default()
                    .push(project.id.clone());
            }

            // Extract task data (this is heavy)
            for task in project.tasks.values() {
                self.ids_to_target_options
                    .entry(project.id.clone())
                    .or_default()
                    .insert(
                        task.target.clone(),
                        // Only copy fields needed for task deps resolution
                        TaskOptions {
                            allow_failure: task.options.allow_failure,
                            run_in_ci: task.options.run_in_ci.clone(),
                            persistent: task.options.persistent,
                            ..Default::default()
                        },
                    );
            }

            self.insert_or_update_node(project);
        }

        Ok(())
    }

    /// Determine the repository type/structure based on the number of project
    /// sources, and where the point to.
    fn determine_repo_type(&mut self, projects_data: &ProjectBuildDataMap) -> miette::Result<()> {
        let single_project = projects_data.len() == 1;
        let mut has_root_project = false;
        let mut root_project_id = None;

        for (id, build_data) in projects_data {
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
            self.root_id = root_project_id;
        }

        Ok(())
    }

    /// Enforce project constraints and boundaries after all nodes have been inserted.
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

        for (project_index, project_state) in self.graph.node_references() {
            let NodeState::Loaded(project) = project_state else {
                continue;
            };

            let deps: Vec<_> = self
                .graph
                .graph()
                .neighbors_directed(project_index, Direction::Outgoing)
                .flat_map(|dep_index| {
                    self.graph.node_weight(dep_index).and_then(|dep| {
                        match dep {
                            NodeState::Loading => None,
                            NodeState::Loaded(dep) => {
                                Some((
                                    dep,
                                    // Is this safe?
                                    self.graph
                                        .find_edge(project_index, dep_index)
                                        .and_then(|ei| self.graph.edge_weight(ei))
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

    /// Load the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    #[instrument(skip(self))]
    async fn load(&mut self) -> miette::Result<ProjectBuildDataMap> {
        let context = self.context();
        let mut glob_format = WorkspaceProjectGlobFormat::default();
        let mut globs = vec![];
        let mut sources = vec![];

        // Gather all project sources
        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.push((
                    id.to_owned(),
                    WorkspaceRelativePathBuf::from(source.trim_start_matches("./")),
                ));
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
                glob_format = cfg.glob_format;
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

            locate_projects_with_globs(&context, &globs, &mut sources, glob_format)?;
        }

        // Load projects and configs first
        let mut build_data = self.load_build_data(sources).await?;

        // Then extend projects with plugins
        self.extend_build_data(&mut build_data).await?;

        // Validate the default project exists
        if let Some(default_id) = &context.workspace_config.default_project
            && !build_data.contains_key(default_id)
        {
            return Err(ProjectGraphError::InvalidDefaultId {
                id: default_id.to_string(),
            }
            .into());
        }

        // Free up some memory
        mem::take(&mut self.renamed_ids);

        Ok(build_data)
    }

    #[instrument(skip(self))]
    async fn load_build_data(
        &mut self,
        sources: Vec<(Id, WorkspaceRelativePathBuf)>,
    ) -> miette::Result<ProjectBuildDataMap> {
        let context = self.context();
        let config_label = context.config_loader.get_debug_label("moon");
        let config_names = context.config_loader.get_project_file_names();
        let mut projects_data = FxHashMap::<Id, ProjectBuildData>::default();
        let mut dupe_original_ids = FxHashSet::default();

        debug!("Loading projects");

        let mut set = JoinSet::new();

        for (id, source) in sources {
            trace!(
                project_id = id.as_str(),
                "Attempting to load {} (optional)",
                color::file(source.join(&config_label))
            );

            // Hash all project-level config files
            for name in &config_names {
                self.config_paths.insert(source.join(name));
            }

            // Load each project config in parallel
            let context = Arc::clone(&context);

            set.spawn_blocking(move || load_project_build_data(context, id, source));
        }

        while let Some(result) = set.join_next().await {
            let mut build_data = result.into_diagnostic()??;

            // Track ID renames
            if let Some((old_id, new_id)) = build_data.rename_id_if_configured() {
                self.track_id_rename(old_id, new_id, &mut dupe_original_ids, &mut build_data);
            }

            let id = build_data.id.take().expect("Missing project ID!");

            // Check for duplicate IDs
            if let Some(existing_data) = projects_data.get(&id)
                && existing_data.source != build_data.source
            {
                return Err(WorkspaceBuilderError::DuplicateProjectId {
                    id: id.to_string(),
                    old_source: existing_data.source.to_string(),
                    new_source: build_data.source.to_string(),
                }
                .into());
            }

            projects_data.insert(id, build_data);
        }

        if !dupe_original_ids.is_empty() {
            trace!(
                original_ids = ?dupe_original_ids.iter().collect::<Vec<_>>(),
                "Found multiple renamed projects with the same original ID; will ignore these IDs within lookups"
            );

            for dupe_id in dupe_original_ids {
                self.renamed_ids.remove(&dupe_id);
            }
        }

        debug!("Loaded {} projects", projects_data.len());

        Ok(projects_data)
    }

    #[instrument(skip(self))]
    async fn extend_build_data(
        &mut self,
        projects_data: &mut ProjectBuildDataMap,
    ) -> miette::Result<()> {
        debug!("Extending project graph with plugins");

        let outputs = extend_project_build_data_with_plugins(
            self.context(),
            projects_data
                .iter()
                .map(|(id, build_data)| (id.clone(), build_data.source.to_string()))
                .collect::<BTreeMap<_, _>>(),
        )
        .await?;

        let context = self.context();

        for (plugin_id, output, is_toolchain) in outputs {
            let inherit_aliases = if is_toolchain {
                context
                    .toolchains_config
                    .get_plugin_config(&plugin_id)
                    .is_none_or(|cfg| cfg.inherit_aliases)
            } else {
                true
            };

            for (project_id, mut project_extend) in output.extended_projects {
                if !projects_data.contains_key(&project_id) {
                    return Err(ProjectGraphError::UnconfiguredID {
                        id: project_id.to_string(),
                    }
                    .into());
                }

                if inherit_aliases && let Some(alias) = project_extend.alias.take() {
                    self.track_alias(&project_id, alias, &plugin_id, projects_data)?;
                }

                if let Some(build_data) = projects_data.get_mut(&project_id) {
                    build_data.extensions.push(project_extend);
                }
            }

            for input_file in output.input_files {
                self.config_paths.insert(
                    context
                        .toolchain_registry
                        .from_virtual_path(input_file)
                        .relative_to(&context.workspace_root)
                        .into_diagnostic()?,
                );
            }
        }

        debug!("Loaded {} project aliases", self.aliases_to_ids.len());

        Ok(())
    }

    fn track_alias(
        &mut self,
        id: &Id,
        alias: String,
        plugin_id: &Id,
        projects_data: &mut ProjectBuildDataMap,
    ) -> miette::Result<()> {
        // Skip aliases that match its own ID
        if alias == id.as_str() {
            return Ok(());
        }

        // Skip aliases that are an invalid ID format
        if let Err(error) = Id::new(&alias) {
            debug!(
                "Skipping alias {} for project {} as its an invalid format: {error}",
                color::label(&alias),
                color::id(id),
            );

            return Ok(());
        }

        // Skip aliases that would override an ID
        if projects_data.contains_key(alias.as_str()) {
            debug!(
                "Skipping alias {} for project {} as it conflicts with the existing project {}",
                color::label(&alias),
                color::id(id),
                color::id(&alias),
            );

            return Ok(());
        }

        // Skip aliases that collide with another alias
        if let Some(existing_id) = self.aliases_to_ids.get(&alias) {
            // Skip if the existing ID is already for this ID.
            // This scenario is possible when multiple toolchains
            // extract the same aliases (Bun vs Node, etc).
            if existing_id == id {
                return Ok(());
            }

            debug!(
                "Skipping alias {} for project {} as it already exists for project {}",
                color::label(&alias),
                color::id(id),
                color::id(existing_id),
            );

            return Ok(());
        }

        projects_data
            .get_mut(id)
            .expect("Project build data not found!")
            .aliases
            .insert(alias.clone(), plugin_id.to_owned());

        self.aliases_to_ids.insert(alias, id.to_owned());

        Ok(())
    }

    fn track_id_rename(
        &mut self,
        old_id: Id,
        new_id: Id,
        duplicate_ids: &mut FxHashSet<Id>,
        project_data: &mut ProjectBuildData,
    ) {
        debug!(
            old_id = old_id.as_str(),
            new_id = new_id.as_str(),
            "Project has been configured with an explicit identifier of {}, renaming from {}",
            color::id(&new_id),
            color::id(&old_id),
        );

        if self.renamed_ids.contains_key(&old_id) {
            duplicate_ids.insert(old_id.clone());
        } else {
            self.renamed_ids.insert(old_id.clone(), new_id.clone());
        }

        project_data.id = Some(new_id);
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
