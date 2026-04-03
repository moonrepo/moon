use crate::projects_builder::*;
use crate::projects_locator::locate_projects_with_globs;
use crate::repo_type::RepoType;
use crate::tasks_builder::*;
use crate::tasks_querent::*;
use crate::workspace_builder::*;
use crate::workspace_builder_error::WorkspaceBuilderError;
use crate::workspace_cache::*;
use daggy::Dag;
use miette::IntoDiagnostic;
use moon_cache::CacheEngine;
use moon_common::{
    Id, color,
    path::{PathExt, WorkspaceRelativePathBuf, is_root_level_source},
};
use moon_config::{
    ConfigLoader, DependencyScope, ExtensionsConfig, InheritedTasksManager,
    ProjectDependencyConfig, TaskDependencyType, ToolchainsConfig, WorkspaceConfig,
    WorkspaceProjectGlobFormat, WorkspaceProjects, finalize_config,
};
use moon_extension_plugin::ExtensionRegistry;
use moon_pdk_api::{ExtendProjectGraphInput, ExtendProjectGraphOutput};
use moon_project::{Project, ProjectAlias, ProjectError};
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
use starbase_utils::glob::{self, GlobWalkOptions};
use starbase_utils::json;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{debug, instrument, trace};

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilderAsync {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: FxHashSet<WorkspaceRelativePathBuf>,

    /// Aliases to their associated project by ID.
    aliases: FxHashMap<String, Id>,

    /// Projects grouped by tag, for use in task dependency resolution.
    projects_by_tag: FxHashMap<Id, Vec<Id>>,

    /// Mapping of project IDs to associated data required for building
    /// the project itself. Data is wiped after building the graph!
    project_data: FxHashMap<Id, ProjectBuildData>,

    /// The project DAG.
    project_graph: Dag<NodeState<Project>, DependencyScope>,

    // Mapping of project IDs to their node index in the graph, for quick lookup.
    project_indexes: FxHashMap<Id, NodeIndex>,

    /// Projects that have explicitly renamed themselves with the `id` setting.
    /// Maps original ID to renamed ID.
    renamed_project_ids: FxHashMap<Id, Id>,

    /// The type of repository: monorepo or polyrepo.
    repo_type: RepoType,

    /// The root project ID (only if a monorepo).
    root_project_id: Option<Id>,

    /// Mapping of task targets to associated data required for building
    /// the project itself. Data is wiped after building the graph!
    task_data: FxHashMap<Target, TaskBuildData>,

    /// The task DAG.
    task_graph: Dag<NodeState<Task>, TaskDependencyType>,

    // Mapping of task targets to their node index in the graph, for quick lookup.
    task_indexes: FxHashMap<Target, NodeIndex>,
}

impl WorkspaceBuilderAsync {
    #[instrument(skip_all)]
    pub async fn new(context: WorkspaceBuilderContext) -> miette::Result<WorkspaceBuilderAsync> {
        debug!("Building workspace graph (project and task graphs)");

        let mut graph = WorkspaceBuilderAsync {
            config_paths: FxHashSet::default(),
            context: Some(Arc::new(context)),
            aliases: FxHashMap::default(),
            projects_by_tag: FxHashMap::default(),
            project_data: FxHashMap::default(),
            project_graph: Dag::new(),
            project_indexes: FxHashMap::default(),
            renamed_project_ids: FxHashMap::default(),
            repo_type: RepoType::Unknown,
            root_project_id: None,
            task_data: FxHashMap::default(),
            task_graph: Dag::new(),
            task_indexes: FxHashMap::default(),
        };

        graph.preload_build_data().await?;
        graph.determine_repo_type()?;

        Ok(graph)
    }

    /// Load and build all projects into the graph, as configured in the workspace.
    pub async fn build_project_graph(&mut self) -> miette::Result<()> {
        let context = self.context();
        let monorepo = self.repo_type.is_monorepo();
        let mut set = JoinSet::new();
        let (tx, mut rx) = mpsc::channel::<ProjectBuildEvent>(1000);

        // Build each project in a separate task
        for (id, build_data) in mem::take(&mut self.project_data) {
            debug!(
                project_id = id.as_str(),
                "Building project {}",
                color::id(&id)
            );

            let context = Arc::clone(&context);
            let root_id = self.root_project_id.clone();
            let tx = tx.clone();

            set.spawn(async move {
                build_project(context, build_data, id, root_id, monorepo, tx).await
            });
        }

        // Receive events from each background task
        while let Some(event) = rx.recv().await {
            match event {
                ProjectBuildEvent::Node(project) => {
                    // Extract tags and group projects
                    for tag in &project.config.tags {
                        self.projects_by_tag
                            .entry(tag.to_owned())
                            .or_default()
                            .push(project.id.clone());
                    }

                    // Extract task build data
                    for task in project.tasks.values() {
                        self.task_data.insert(
                            task.target.clone(),
                            TaskBuildData {
                                options: task.options.clone(),
                                ..Default::default()
                            },
                        );
                    }

                    insert_or_update_project_node(
                        project,
                        &mut self.project_graph,
                        &mut self.project_indexes,
                    );
                }
                ProjectBuildEvent::Edge(from_id, to_id, scope) => {
                    let from_index = get_or_insert_project_node(
                        &from_id,
                        &mut self.project_graph,
                        &mut self.project_indexes,
                    );

                    let to_index = get_or_insert_project_node(
                        &to_id,
                        &mut self.project_graph,
                        &mut self.project_indexes,
                    );

                    self.project_graph
                        .add_edge(from_index, to_index, scope)
                        .map_err(|_| ProjectGraphError::WouldCycle {
                            source_id: from_id.to_string(),
                            target_id: to_id.to_string(),
                        })?;
                }
            }
        }

        // Ensure all background tasks have completed
        set.join_all().await;

        Ok(())
    }

    /// Load and build all tasks into the graph, as configured in the workspace.
    pub async fn build_task_graph(&mut self) -> miette::Result<()> {
        let context = self.context();
        let mut set = JoinSet::new();
        let (tx, mut rx) = mpsc::channel::<TaskBuildEvent>(1000);

        // Build each task in a separate task
        for (target, _build_data) in mem::take(&mut self.task_data) {
            debug!(
                task_target = target.as_str(),
                "Building task {}",
                color::id(&target)
            );

            // Extract the task from the project, as the data will live
            // in the task graph and not the project graph
            let Some(project_index) = self.project_indexes.get(target.get_project_id()?) else {
                panic!("Unable to load task, owning project does not exist!");
            };

            let Some(NodeState::Loaded(project)) =
                self.project_graph.node_weight_mut(*project_index)
            else {
                panic!("Unable to load task, owning project is in a non-loaded state!");
            };

            let mut task = project.tasks.remove(&target.task_id).unwrap();

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

            let context = Arc::clone(&context);
            let tx = tx.clone();

            set.spawn(async move { build_task(context, task, tx).await });
        }

        // Receive events from each background task
        while let Some(event) = rx.recv().await {
            match event {
                TaskBuildEvent::Node(task) => {
                    insert_or_update_task_node(task, &mut self.task_graph, &mut self.task_indexes);
                }
                TaskBuildEvent::Edge(from_target, to_target, scope) => {
                    let from_index = get_or_insert_task_node(
                        &from_target,
                        &mut self.task_graph,
                        &mut self.task_indexes,
                    );

                    let to_index = get_or_insert_task_node(
                        &to_target,
                        &mut self.task_graph,
                        &mut self.task_indexes,
                    );

                    self.task_graph
                        .add_edge(from_index, to_index, scope)
                        .map_err(|_| ProjectGraphError::WouldCycle {
                            source_id: from_target.to_string(),
                            target_id: to_target.to_string(),
                        })?;
                }
            }
        }

        // Ensure all background tasks have completed
        set.join_all().await;

        Ok(())
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
                .graph()
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

    /// Preload the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    async fn preload_build_data(&mut self) -> miette::Result<()> {
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
        self.load_project_build_data(sources).await?;

        // Then extend projects from toolchains
        self.extend_project_build_data().await?;

        // Include all workspace-level config files
        let ext_glob = context.config_loader.get_ext_glob();

        for file in glob::walk_fast_with_options(
            &context.config_loader.dir,
            [&format!("*.{ext_glob}"), &format!("tasks/**/*.{ext_glob}")],
            GlobWalkOptions::default().cache().log_results(),
        )? {
            self.config_paths.insert(
                file.relative_to(&context.workspace_root)
                    .into_diagnostic()?,
            );
        }

        // Validate the default project exists
        if let Some(default_id) = &context.workspace_config.default_project
            && !self.project_data.contains_key(default_id)
        {
            return Err(ProjectGraphError::InvalidDefaultId {
                id: default_id.to_string(),
            }
            .into());
        }

        Ok(())
    }

    async fn load_project_build_data(
        &mut self,
        sources: Vec<(Id, WorkspaceRelativePathBuf)>,
    ) -> miette::Result<()> {
        let context = self.context();
        let config_label = context.config_loader.get_debug_label("moon");
        let config_names = context.config_loader.get_project_file_names();
        let mut project_data: FxHashMap<Id, ProjectBuildData> = FxHashMap::default();
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
                self.track_project_id_rename(&old_id, &new_id, &mut dupe_original_ids);

                build_data.original_id = Some(old_id);
                build_data.id = Some(new_id);
            }

            let id = build_data.id.take().expect("Missing project ID!");

            // Check for duplicate IDs
            if let Some(existing_data) = project_data.get(&id)
                && existing_data.source != build_data.source
            {
                return Err(WorkspaceBuilderError::DuplicateProjectId {
                    id: id.to_string(),
                    old_source: existing_data.source.to_string(),
                    new_source: build_data.source.to_string(),
                }
                .into());
            }

            project_data.insert(id, build_data);
        }

        if !dupe_original_ids.is_empty() {
            trace!(
                original_ids = ?dupe_original_ids.iter().collect::<Vec<_>>(),
                "Found multiple renamed projects with the same original ID; will ignore these IDs within lookups"
            );

            for dupe_id in dupe_original_ids {
                self.renamed_project_ids.remove(&dupe_id);
            }
        }

        debug!("Loaded {} projects", project_data.len());

        self.project_data.extend(project_data);

        Ok(())
    }

    async fn extend_project_build_data(&mut self) -> miette::Result<()> {
        let context = self.context();

        debug!("Extending project graph");

        let project_sources = self
            .project_data
            .iter()
            .map(|(id, build_data)| (id.clone(), build_data.source.to_string()))
            .collect::<BTreeMap<_, _>>();

        let outputs = tokio::spawn(extend_project_build_data_with_plugins(
            context,
            project_sources,
        ))
        .await
        .into_diagnostic()??;
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
                if !self.project_data.contains_key(&project_id) {
                    return Err(ProjectGraphError::UnconfiguredID {
                        id: project_id.to_string(),
                    }
                    .into());
                }

                if inherit_aliases && let Some(alias) = project_extend.alias.take() {
                    self.track_project_alias(&project_id, alias, &plugin_id)?;
                }

                if let Some(build_data) = self.project_data.get_mut(&project_id) {
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

        debug!("Loaded {} project aliases", self.aliases.len());

        Ok(())
    }

    fn track_project_alias(
        &mut self,
        id: &Id,
        alias: String,
        plugin_id: &Id,
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
            if existing_id == id {
                return Ok(());
            }

            debug!(
                "Skipping alias {} for project {} as it already exists for project {}",
                color::label(&alias),
                color::id(&id),
                color::id(existing_id),
            );

            return Ok(());
        }

        self.project_data
            .get_mut(id)
            .expect("Project build data not found!")
            .aliases
            .insert(alias.clone(), plugin_id.to_owned());

        self.aliases.insert(alias, id.to_owned());

        Ok(())
    }

    fn track_project_id_rename(
        &mut self,
        old_id: &Id,
        new_id: &Id,
        duplicate_ids: &mut FxHashSet<Id>,
    ) {
        debug!(
            old_id = old_id.as_str(),
            new_id = new_id.as_str(),
            "Project has been configured with an explicit identifier of {}, renaming from {}",
            color::id(new_id),
            color::id(old_id),
        );

        if self.renamed_project_ids.contains_key(old_id) {
            duplicate_ids.insert(old_id.to_owned());
        } else {
            self.renamed_project_ids
                .insert(old_id.to_owned(), new_id.to_owned());
        }
    }

    fn context(&self) -> Arc<WorkspaceBuilderContext> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
