use crate::project_events::{ExtendProjectEvent, ExtendProjectGraphEvent};
use crate::project_graph::{GraphType, ProjectGraph, ProjectNode};
use crate::project_graph_cache::ProjectsState;
use crate::project_graph_error::ProjectGraphError;
use crate::project_graph_hash::ProjectGraphHash;
use crate::projects_locator::locate_projects_with_globs;
use async_recursion::async_recursion;
use moon_cache::CacheEngine;
use moon_common::path::{to_virtual_string, WorkspaceRelativePathBuf};
use moon_common::{color, consts, is_test_env, Id};
use moon_config::{
    DependencyScope, InheritedTasksManager, ProjectConfig, ProjectsSourcesList, ToolchainConfig,
    WorkspaceConfig, WorkspaceProjects,
};
use moon_hash::HashEngine;
use moon_project::Project;
use moon_project_builder::{DetectLanguageEvent, ProjectBuilder, ProjectBuilderContext};
use moon_project_constraints::{enforce_project_type_relationships, enforce_tag_relationships};
use moon_task_builder::DetectPlatformEvent;
use moon_vcs::BoxedVcs;
use petgraph::graph::DiGraph;
use petgraph::prelude::NodeIndex;
use petgraph::Direction;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use starbase_events::Emitter;
use starbase_utils::{glob, json};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, trace};

pub struct ProjectGraphBuilderContext<'app> {
    pub extend_project: Emitter<ExtendProjectEvent>,
    pub extend_project_graph: Emitter<ExtendProjectGraphEvent>,
    pub detect_language: Emitter<DetectLanguageEvent>,
    pub detect_platform: Emitter<DetectPlatformEvent>,
    // pub extend_project: &'app Emitter<ExtendProjectEvent>,
    // pub extend_project_graph: &'app Emitter<ExtendProjectGraphEvent>,
    // pub detect_language: &'app Emitter<DetectLanguageEvent>,
    // pub detect_platform: &'app Emitter<DetectPlatformEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub toolchain_config: &'app ToolchainConfig,
    pub vcs: Option<&'app BoxedVcs>,
    pub working_dir: &'app Path,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

#[derive(Deserialize, Serialize)]
pub struct ProjectGraphBuilder<'app> {
    #[serde(skip)]
    context: Option<Arc<ProjectGraphBuilderContext<'app>>>,

    /// Mapping of project IDs to project aliases.
    aliases: FxHashMap<Id, String>,

    /// Loaded project configuration (`moon.yml`) files.
    #[serde(skip)]
    configs: FxHashMap<Id, ProjectConfig>,

    /// The DAG instance.
    graph: GraphType,

    /// Nodes (projects) inserted into the graph.
    nodes: FxHashMap<Id, NodeIndex>,

    /// Projects that have explicitly renamed themselves.
    /// Maps original ID to renamed ID.
    renamed_ids: FxHashMap<Id, Id>,

    /// The root project ID.
    root_id: Option<Id>,

    /// Mapping of project IDs to file system sources,
    /// derived from the `workspace.projects` setting.
    sources: FxHashMap<Id, WorkspaceRelativePathBuf>,
}

impl<'app> ProjectGraphBuilder<'app> {
    /// Create a new project graph instance without reading from the
    /// cache, and preloading all project sources and aliases.
    pub async fn new(
        context: ProjectGraphBuilderContext<'app>,
    ) -> miette::Result<ProjectGraphBuilder<'app>> {
        debug!("Building project graph");

        let mut graph = ProjectGraphBuilder {
            context: Some(Arc::new(context)),
            configs: FxHashMap::default(),
            aliases: FxHashMap::default(),
            graph: DiGraph::new(),
            nodes: FxHashMap::default(),
            renamed_ids: FxHashMap::default(),
            root_id: None,
            sources: FxHashMap::default(),
        };

        graph.preload().await?;

        Ok(graph)
    }

    /// Create a project graph with all projects inserted as nodes,
    /// and read from the file system cache when applicable.
    pub async fn generate(
        context: ProjectGraphBuilderContext<'app>,
        cache_engine: &CacheEngine,
        hash_engine: &HashEngine,
    ) -> miette::Result<ProjectGraphBuilder<'app>> {
        let is_vcs_enabled = context
            .vcs
            .as_ref()
            .expect("VCS is required for project graph caching!")
            .is_enabled();
        let mut graph = Self::new(context).await?;

        // No VCS to hash with, so abort caching
        if !is_vcs_enabled {
            graph.load_all().await?;

            return Ok(graph);
        }

        // Hash the project graph based on the preloaded state
        let mut graph_contents = ProjectGraphHash::new();
        graph_contents.add_sources(&graph.sources);
        graph_contents.add_aliases(&graph.aliases);
        graph_contents.add_configs(graph.hash_required_configs().await?);

        let hash = hash_engine.save_manifest_without_hasher("Project graph", &graph_contents)?;

        debug!(hash, "Generated hash for project graph");

        // Check the current state and cache
        let mut state = cache_engine.cache_state::<ProjectsState>("projects.json")?;
        let cache_path = cache_engine.states_dir.join("partialProjectGraph.json");

        if hash == state.data.last_hash && cache_path.exists() {
            debug!(
                cache = ?cache_path,
                "Loading project graph with {} projects from cache",
                graph.sources.len(),
            );

            let mut cache: ProjectGraphBuilder = json::read_file(cache_path)?;
            cache.configs = graph.configs;
            cache.context = graph.context;

            return Ok(cache);
        }

        // Build the graph, update the state, and save the cache
        debug!(
            "Generating project graph with {} projects",
            graph.sources.len(),
        );

        graph.load_all().await?;

        state.data.last_hash = hash;
        state.data.projects = graph.sources.clone();
        state.save()?;

        json::write_file(cache_path, &graph, false)?;

        Ok(graph)
    }

    /// Build the project graph and return a new structure.
    pub async fn build(mut self) -> miette::Result<ProjectGraph> {
        self.enforce_constraints()?;

        let context = self.context.take().unwrap();
        let mut nodes = FxHashMap::default();

        for (id, index) in self.nodes {
            let source = self.sources.remove(&id).unwrap();

            nodes.insert(
                id,
                ProjectNode {
                    index,
                    source,
                    ..ProjectNode::default()
                },
            );
        }

        for (id, alias) in self.aliases {
            nodes.entry(id).and_modify(|node| {
                node.alias = Some(alias);
            });
        }

        for (original_id, id) in self.renamed_ids {
            nodes.entry(id).and_modify(|node| {
                node.original_id = Some(original_id);
            });
        }

        let mut graph = ProjectGraph::new(self.graph, nodes, context.workspace_root);

        graph.working_dir = context.working_dir.to_owned();

        graph.check_boundaries =
            !is_test_env() && context.workspace_config.experiments.task_output_boundaries;

        Ok(graph)
    }

    /// Load a single project by name or alias into the graph.
    pub async fn load(&mut self, project_locator: &str) -> miette::Result<()> {
        self.internal_load(project_locator, &mut FxHashSet::default())
            .await?;

        Ok(())
    }

    /// Load all projects into the graph, as configured in the workspace.
    pub async fn load_all(&mut self) -> miette::Result<()> {
        let ids = self.sources.keys().cloned().collect::<Vec<_>>();

        for id in ids {
            self.internal_load(&id, &mut FxHashSet::default()).await?;
        }

        Ok(())
    }

    #[async_recursion]
    async fn internal_load(
        &mut self,
        project_locator: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<(Id, NodeIndex)> {
        let id = self.resolve_id(project_locator);

        // Already loaded, exit early with existing index
        if let Some(index) = self.nodes.get(&id) {
            trace!(
                id = id.as_str(),
                "Project already exists in the project graph, skipping load",
            );

            return Ok((id, *index));
        }

        // Check that the project ID is configured
        trace!(
            id = id.as_str(),
            "Project does not exist in the project graph, attempting to load",
        );

        let Some(source) = self.sources.get(&id).map(|s| s.to_owned()) else {
            return Err(ProjectGraphError::UnconfiguredID(id).into());
        };

        // Create the project
        let mut project = self.build_project(id, source).await?;
        let id = project.id.clone();

        cycle.insert(id.clone());

        // Create dependent projects
        let mut edges = vec![];

        for dep_config in &mut project.dependencies {
            let loaded_dep_id = if cycle.contains(&dep_config.id) {
                debug!(
                    id = id.as_str(),
                    dependency_id = dep_config.id.as_str(),
                    "Encountered a dependency cycle (from project); will disconnect nodes to avoid recursion",
                );

                continue;

                // Don't link the root project to any project, but still load it
            } else if matches!(dep_config.scope, DependencyScope::Root) {
                self.internal_load(&dep_config.id, cycle).await?.0

                // Otherwise link projects
            } else {
                let dep = self.internal_load(&dep_config.id, cycle).await?;
                edges.push((dep.1, dep_config.scope));
                dep.0
            };

            if loaded_dep_id != dep_config.id {
                dep_config.id = loaded_dep_id;
            }
        }

        // Add to the graph
        let index = self.graph.add_node(project);

        self.nodes.insert(id.clone(), index);

        for edge in edges {
            self.graph.add_edge(index, edge.0, edge.1);
        }

        cycle.clear();

        Ok((id, index))
    }

    /// Create and build the project with the provided ID and source.
    async fn build_project(
        &mut self,
        id: Id,
        source: WorkspaceRelativePathBuf,
    ) -> miette::Result<Project> {
        debug!(id = id.as_str(), "Building project {}", color::id(&id));

        let context = self.context();

        if !source.to_path(context.workspace_root).exists() {
            return Err(ProjectGraphError::MissingAtSource(source.to_string()).into());
        }

        let mut builder = ProjectBuilder::new(
            &id,
            &source,
            ProjectBuilderContext {
                detect_language: &context.detect_language,
                detect_platform: &context.detect_platform,
                root_project_id: self.root_id.as_ref(),
                toolchain_config: context.toolchain_config,
                workspace_root: context.workspace_root,
            },
        )?;

        if let Some(config) = self.configs.remove(&id) {
            builder.inherit_local_config(config).await?;
        } else {
            builder.load_local_config().await?;
        }

        builder.inherit_global_config(context.inherited_tasks)?;

        let extended_data = context
            .extend_project
            .emit(ExtendProjectEvent {
                project_id: id.to_owned(),
                project_source: source.to_owned(),
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
        if let Some(alias) = self.aliases.get(&id) {
            builder.set_alias(alias);
        }

        let project = builder.build().await?;

        Ok(project)
    }

    /// Enforce project constraints and boundaries after all nodes have been inserted.
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

        for project in self.graph.node_weights() {
            let deps: Vec<_> = self
                .graph
                .neighbors_directed(*self.nodes.get(&project.id).unwrap(), Direction::Outgoing)
                .map(|idx| self.graph.node_weight(idx).unwrap())
                .collect();

            for dep in deps {
                if type_relationships {
                    enforce_project_type_relationships(project, dep)?;
                }

                for (source_tag, required_tags) in tag_relationships {
                    enforce_tag_relationships(project, source_tag, dep, required_tags)?;
                }
            }
        }

        Ok(())
    }

    /// When caching the project graph, we must hash all project and workspace
    /// config files that are required to invalidate the cache.
    async fn hash_required_configs(
        &self,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let context = self.context();
        let mut configs = vec![];

        // Hash all project-level config files
        for source in self.sources.values() {
            configs.push(
                source
                    .join(consts::CONFIG_PROJECT_FILENAME)
                    .as_str()
                    .to_owned(),
            );
        }

        // Hash all workspace-level config files
        for file in glob::walk(
            context.workspace_root.join(consts::CONFIG_DIRNAME),
            ["*.yml", "tasks/**/*.yml"],
        )? {
            configs.push(to_virtual_string(
                file.strip_prefix(context.workspace_root).unwrap(),
            )?);
        }

        context
            .vcs
            .as_ref()
            .unwrap()
            .get_file_hashes(&configs, true, 500)
            .await
    }

    /// Preload the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    async fn preload(&mut self) -> miette::Result<()> {
        let context = self.context();
        let mut globs = vec![];
        let mut sources = vec![];

        // Locate all project sources
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

            locate_projects_with_globs(context.workspace_root, &globs, &mut sources, context.vcs)?;
        }

        // Extend graph from subscribers
        debug!("Extending project graph from subscribers");

        let aliases = context
            .extend_project_graph
            .emit(ExtendProjectGraphEvent {
                sources: sources.clone(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?
            .aliases;

        // Load all config files
        self.preload_configs(&mut sources)?;

        // Find the root project
        self.root_id = sources.iter().find_map(|(id, source)| {
            if source.as_str().is_empty() || source.as_str() == "." {
                Some(id.to_owned())
            } else {
                None
            }
        });

        // Set our data and warn/error against problems
        for (id, source) in sources {
            if let Some(existing_source) = self.sources.get(&id) {
                if existing_source == &source {
                    continue;
                }

                return Err(ProjectGraphError::DuplicateId {
                    id: id.clone(),
                    old_source: existing_source.to_string(),
                    new_source: source.to_string(),
                }
                .into());
            } else {
                self.sources.insert(id, source);
            }
        }

        let mut dupe_aliases = FxHashMap::<String, Id>::default();

        for (id, alias) in aliases {
            let id = match self.renamed_ids.get(&id) {
                Some(new_id) => new_id,
                None => &id,
            };

            // Skip aliases that match its own ID
            if id == &alias {
                continue;
            }

            // Skip aliases that would override an ID
            if self.sources.contains_key(&alias) {
                debug!(
                    "Skipping alias {} (for project {}) as it conflicts with the project {}",
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

                if self
                    .context()
                    .workspace_config
                    .experiments
                    .strict_project_aliases
                {
                    return Err(ProjectGraphError::DuplicateAlias {
                        alias: alias.clone(),
                        old_id: existing_id.to_owned(),
                        new_id: id.clone(),
                    }
                    .into());
                } else {
                    debug!(
                        duplicate_id = id.as_str(),
                        existing_id = existing_id.as_str(),
                        "Skipping duplicate alias {} for project {} to avoid conflicts",
                        color::label(&alias),
                        color::id(id),
                    );

                    continue;
                }
            }

            dupe_aliases.insert(alias.clone(), id.to_owned());
            self.aliases.insert(id.to_owned(), alias);
        }

        Ok(())
    }

    fn preload_configs(&mut self, sources: &mut ProjectsSourcesList) -> miette::Result<()> {
        let context = self.context();
        let mut configs = FxHashMap::default();
        let mut renamed_ids = FxHashMap::default();

        for (id, source) in sources {
            let config_name = source.join(consts::CONFIG_PROJECT_FILENAME);
            let config_path = config_name.to_path(context.workspace_root);

            debug!(
                id = id.as_str(),
                file = ?config_path,
                "Attempting to load {} (optional)",
                color::file(config_name.as_str())
            );

            let config = ProjectConfig::load(context.workspace_root, config_path)?;

            // Track ID renames
            if let Some(new_id) = &config.id {
                if new_id != id {
                    renamed_ids.insert(id.to_owned(), new_id.to_owned());
                    *id = new_id.to_owned();
                }
            }

            configs.insert(config.id.clone().unwrap_or(id.to_owned()), config);
        }

        self.configs.extend(configs);
        self.renamed_ids.extend(renamed_ids);

        Ok(())
    }

    fn context(&self) -> Arc<ProjectGraphBuilderContext<'app>> {
        Arc::clone(self.context.as_ref().unwrap())
    }

    fn resolve_id(&self, project_locator: &str) -> Id {
        let id = if self.sources.contains_key(project_locator) {
            Id::raw(project_locator)
        } else {
            match self.aliases.iter().find_map(|(id, alias)| {
                if alias == project_locator {
                    Some(id)
                } else {
                    None
                }
            }) {
                Some(project_id) => project_id.to_owned(),
                None => Id::raw(project_locator),
            }
        };

        match self.renamed_ids.get(&id) {
            Some(new_id) => new_id.to_owned(),
            None => id,
        }
    }
}
