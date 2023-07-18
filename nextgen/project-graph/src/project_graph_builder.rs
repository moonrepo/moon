use crate::project_events::ExtendProjectEvent;
use crate::project_events::ExtendProjectGraphEvent;
use crate::project_graph::{GraphType, ProjectGraph, ProjectNode};
use crate::project_graph_error::ProjectGraphError;
use crate::project_graph_hash::ProjectGraphHash;
use crate::projects_locator::locate_projects_with_globs;
use async_recursion::async_recursion;
use moon_cache::CacheEngine;
use moon_common::path::{to_virtual_string, WorkspaceRelativePath, WorkspaceRelativePathBuf};
use moon_common::{color, consts, Id};
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig, WorkspaceProjects};
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
use starbase_events::Emitter;
use starbase_utils::{glob, json};
use std::collections::BTreeMap;
use std::mem;
use std::path::Path;
use tracing::{debug, trace, warn};

pub struct ProjectGraphBuilderContext<'app> {
    pub extend_project: &'app Emitter<ExtendProjectEvent>,
    pub extend_project_graph: &'app Emitter<ExtendProjectGraphEvent>,
    pub detect_language: &'app Emitter<DetectLanguageEvent>,
    pub detect_platform: &'app Emitter<DetectPlatformEvent>,
    pub inherited_tasks: &'app InheritedTasksManager,
    pub toolchain_config: &'app ToolchainConfig,
    pub vcs: &'app BoxedVcs,
    pub workspace_config: &'app WorkspaceConfig,
    pub workspace_root: &'app Path,
}

pub struct ProjectGraphBuilder<'app> {
    context: ProjectGraphBuilderContext<'app>,

    /// Mapping of project aliases to project IDs.
    aliases: FxHashMap<String, Id>,

    /// The DAG instance.
    graph: GraphType,

    /// Nodes (projects) inserted into the graph.
    nodes: FxHashMap<Id, NodeIndex>,

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
        debug!("Creating project graph");

        let mut graph = ProjectGraphBuilder {
            context,
            aliases: FxHashMap::default(),
            graph: DiGraph::new(),
            nodes: FxHashMap::default(),
            sources: FxHashMap::default(),
        };

        graph.preload().await?;

        Ok(graph)
    }

    /// Create a project graph with all projects inserted as nodes,
    /// and read from the file system cache when applicable.
    pub async fn generate(
        context: ProjectGraphBuilderContext<'app>,
        hash_engine: &HashEngine,
        cache_engine: &CacheEngine,
    ) -> miette::Result<ProjectGraphBuilder<'app>> {
        let is_vcs_enabled = context.vcs.is_enabled();
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
        let mut state = cache_engine.cache_projects_state()?;
        let cache_path = cache_engine.get_state_path("partialProjectGraph.json");

        if hash == state.last_hash && cache_path.exists() {
            debug!(
                "Loading project graph with {} projects from cache",
                graph.sources.len(),
            );

            graph.graph = json::read_file(cache_path)?;

            return Ok(graph);
        }

        // Build the graph, update the state, and save the cache
        debug!(
            "Building project graph with {} projects",
            graph.sources.len(),
        );

        graph.load_all().await?;

        state.last_hash = hash;
        state.projects = graph.sources.clone();
        state.save()?;

        json::write_file(cache_path, &graph.graph, false)?;

        Ok(graph)
    }

    /// Build the project graph and return a new structure.
    pub async fn build(self) -> miette::Result<ProjectGraph> {
        self.enforce_constraints()?;

        let mut nodes = FxHashMap::default();

        for (alias, id) in self.aliases {
            nodes.insert(
                id,
                ProjectNode {
                    alias: Some(alias),
                    ..ProjectNode::default()
                },
            );
        }

        for (id, index) in self.nodes {
            nodes
                .entry(id)
                .and_modify(|node| node.index = index)
                .or_insert(ProjectNode {
                    index,
                    ..ProjectNode::default()
                });
        }

        Ok(ProjectGraph {
            graph: self.graph,
            nodes,
        })
    }

    /// Load a single project by ID or alias into the graph.
    pub async fn load(&mut self, alias_or_id: &str) -> miette::Result<()> {
        self.internal_load(alias_or_id, &mut FxHashSet::default())
            .await?;

        Ok(())
    }

    /// Load all projects into the graph, as configured in the workspace.
    pub async fn load_all(&mut self) -> miette::Result<()> {
        let mut sources = FxHashMap::default();

        // Take ownership so that we can mutate while looping,
        // without having to clone all sources.
        for (id, source) in mem::take(&mut self.sources) {
            self.internal_load(&id, &mut FxHashSet::default()).await?;
            sources.insert(id, source);
        }

        self.sources = sources;

        Ok(())
    }

    #[async_recursion]
    async fn internal_load(
        &mut self,
        alias_or_id: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<NodeIndex> {
        let id = self.resolve_id(alias_or_id);

        // Already loaded, exit early with existing index
        if let Some(index) = self.nodes.get(&id) {
            trace!(
                project_id = id.as_str(),
                "Project already exists in the project graph, skipping load",
            );

            return Ok(*index);
        }

        // Check that the project ID is configured
        trace!(
            project_id = id.as_str(),
            "Project does not exist in the project graph, attempting to load",
        );

        let Some(source) = self.sources.get(&id) else {
            return Err(ProjectGraphError::UnconfiguredID(id).into());
        };

        // Create the project
        let project = self.build_project(&id, source).await?;

        cycle.insert(id.clone());

        // Create dependent projects
        let mut edges = vec![];

        for (dep_id, dep_config) in &project.dependencies {
            if cycle.contains(dep_id) {
                warn!(
                    project_id = id.as_str(),
                    dependency_id = dep_id.as_str(),
                    "Encountered a dependency cycle; will disconnect nodes to avoid recursion",
                );
            } else {
                edges.push((self.internal_load(dep_id, cycle).await?, dep_config.scope));
            }
        }

        // Insert into the graph and connect edges
        let index = self.graph.add_node(project);

        for edge in edges {
            self.graph.add_edge(index, edge.0, edge.1);
        }

        self.nodes.insert(id, index);

        Ok(index)
    }

    /// Create and build the project with the provided ID and source.
    async fn build_project(
        &self,
        id: &Id,
        source: &WorkspaceRelativePath,
    ) -> miette::Result<Project> {
        debug!("Creating project {}", color::id(id));

        let mut builder = ProjectBuilder::new(
            id,
            source,
            ProjectBuilderContext {
                detect_language: self.context.detect_language,
                detect_platform: self.context.detect_platform,
                toolchain_config: self.context.toolchain_config,
                workspace_root: self.context.workspace_root,
            },
        )?;

        builder.load_local_config().await?;
        builder.inherit_global_config(self.context.inherited_tasks)?;

        let (event, _) = self
            .context
            .extend_project
            .emit(ExtendProjectEvent {
                project_id: id.to_owned(),
                project_source: source.to_owned(),
                workspace_root: self.context.workspace_root.to_owned(),
                extended_dependencies: vec![],
                extended_tasks: FxHashMap::default(),
            })
            .await?;

        // Inherit implicit dependencies
        for dep_config in event.extended_dependencies {
            builder.extend_with_dependency(dep_config);
        }

        // Inherit inferred tasks
        for (task_id, task_config) in event.extended_tasks {
            builder.extend_with_task(task_id, task_config);
        }

        let mut project = builder.build().await?;

        // Inherit alias (is there a better way to do this?)
        for (alias, project_id) in &self.aliases {
            if project_id == id {
                project.alias = Some(alias.to_owned());
                break;
            }
        }

        Ok(project)
    }

    /// Enforce project constraints and boundaries after all nodes have been inserted.
    fn enforce_constraints(&self) -> miette::Result<()> {
        debug!("Enforcing project constraints");

        let type_relationships = self
            .context
            .workspace_config
            .constraints
            .enforce_project_type_relationships;
        let tag_relationships = &self.context.workspace_config.constraints.tag_relationships;

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
            self.context.workspace_root.join(consts::CONFIG_DIRNAME),
            ["*.yml", "tasks/*.yml"],
        )? {
            configs.push(to_virtual_string(
                file.strip_prefix(self.context.workspace_root).unwrap(),
            )?);
        }

        let hashes = self
            .context
            .vcs
            .get_file_hashes(&configs, false, 500)
            .await?;

        Ok(hashes)
    }

    /// Preload the graph with project sources from the workspace configuration.
    /// If globs are provided, walk the file system and gather sources.
    /// Then extend the graph with aliases, derived from all event subscribers.
    async fn preload(&mut self) -> miette::Result<()> {
        let mut globs = vec![];
        let mut sources = FxHashMap::default();

        // Locate all project sources
        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.insert(id.to_owned(), WorkspaceRelativePathBuf::from(source));
            }
        };

        match &self.context.workspace_config.projects {
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

            locate_projects_with_globs(
                self.context.workspace_root,
                &globs,
                &mut sources,
                Some(self.context.vcs),
            )?;
        }

        // Extend graph from subscribers
        debug!("Extending project graph");

        let (event, _) = self
            .context
            .extend_project_graph
            .emit(ExtendProjectGraphEvent {
                sources,
                workspace_root: self.context.workspace_root.to_owned(),
                extended_aliases: FxHashMap::default(),
            })
            .await?;

        self.sources = event.sources;
        self.aliases = event.extended_aliases;

        Ok(())
    }

    fn resolve_id(&self, alias_or_id: &str) -> Id {
        match self.aliases.get(alias_or_id) {
            Some(project_id) => project_id.to_owned(),
            None => Id::raw(alias_or_id),
        }
    }
}
