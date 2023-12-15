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
    DependencyScope, InheritedTasksManager, ToolchainConfig, WorkspaceConfig, WorkspaceProjects,
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
    context: Option<ProjectGraphBuilderContext<'app>>,

    /// Mapping of project aliases to project IDs.
    aliases: FxHashMap<String, Id>,

    /// The DAG instance.
    graph: GraphType,

    /// Nodes (projects) inserted into the graph.
    nodes: FxHashMap<Id, NodeIndex>,

    /// Projects that have explicitly renamed themselves.
    /// Maps original ID to renamed ID.
    renamed_ids: FxHashMap<Id, Id>,

    /// The root project ID.
    #[serde(skip)]
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
            context: Some(context),
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

            dbg!("READ CACHE", std::fs::read_to_string(&cache_path).unwrap());

            let mut cache: ProjectGraphBuilder = json::read_file(cache_path)?;
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

        for (alias, id) in self.aliases {
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
    #[async_recursion]
    pub async fn load(&mut self, project_locator: &str) -> miette::Result<Id> {
        let (id, index) = self.insert_project(project_locator).await?;
        let mut links = vec![id.clone()];

        // Create dependent projects
        if let Some(dep_ids) = self
            .graph
            .node_weight(index)
            .map(|project| project.dependencies.keys().cloned().collect::<Vec<_>>())
        {
            for dep_id in dep_ids {
                links.push(self.load(&dep_id).await?);
            }
        }

        // Then link edges
        for id in links {
            self.link_dependencies(&id, &mut FxHashSet::default())
                .await?;
        }

        Ok(id)
    }

    /// Load all projects into the graph, as configured in the workspace.
    pub async fn load_all(&mut self) -> miette::Result<()> {
        let ids = self.sources.keys().cloned().collect::<Vec<_>>();

        dbg!("load_all", &ids);

        // Create all the nodes first
        for id in &ids {
            self.insert_project(id).await?;
        }

        // Then link edges
        for id in &ids {
            self.link_dependencies(id, &mut FxHashSet::default())
                .await?;
        }

        Ok(())
    }

    async fn insert_project(&mut self, project_locator: &str) -> miette::Result<(Id, NodeIndex)> {
        let id = self.resolve_id(project_locator);

        dbg!("insert_project", &id, &self.renamed_ids);

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
        let project = self.build_project(id, source).await?;

        // Add to the graph
        let id = project.id.clone();
        let index = self.graph.add_node(project);

        self.nodes.insert(id.clone(), index);

        Ok((id, index))
    }

    async fn link_dependencies(
        &mut self,
        project_locator: &str,
        cycle: &mut FxHashSet<Id>,
    ) -> miette::Result<Option<(Id, NodeIndex)>> {
        let id = self.resolve_id(project_locator);

        let Some(index) = self.nodes.get(&id) else {
            return Ok(None);
        };

        if let Some(project) = self.graph.node_weight_mut(*index) {
            cycle.insert(project.id.clone());

            // for (_, dep_config) in &mut project.dependencies {
            //     if cycle.contains(&dep_config.id) {
            //         debug!(
            //             id = id.as_str(),
            //             dependency_id = dep_config.id.as_str(),
            //             "Encountered a dependency cycle (from project); will disconnect nodes to avoid recursion",
            //         );

            //         continue;

            //         // Don't link the root project to any project
            //     } else if matches!(dep_config.scope, DependencyScope::Root) {
            //         continue;

            //         // Otherwise link projects
            //     } else if let Some((dep_id, dep_index)) =
            //         self.link_dependencies(&dep_config.id, cycle).await?
            //     {
            //         self.graph.add_edge(*index, dep_index, dep_config.scope);

            //         if dep_id != dep_config.id {
            //             dep_config.id = dep_id;
            //         }
            //     };
            // }

            cycle.clear();

            return Ok(Some((id, *index)));
        };

        Ok(None)
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
                legacy_task_inheritance: !context
                    .workspace_config
                    .experiments
                    .interweaved_task_inheritance,
                root_project_id: self.root_id.as_ref(),
                toolchain_config: context.toolchain_config,
                workspace_root: context.workspace_root,
            },
        )?;

        builder.load_local_config().await?;
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
        for (alias, project_id) in &self.aliases {
            if project_id == &id {
                builder.set_alias(alias);
                break;
            }
        }

        let project = builder.build().await?;

        // If we received a new ID, update references
        if project.id != id {
            self.sources.remove(&id);
            self.sources.insert(project.id.clone(), source.to_owned());

            if let Some(alias) = project.alias.as_ref() {
                self.aliases.insert(alias.to_owned(), project.id.clone());
            }

            self.renamed_ids.insert(id, project.id.clone());
        }

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
        let mut sources = FxHashMap::default();

        // Locate all project sources
        let mut add_sources = |map: &FxHashMap<Id, String>| {
            for (id, source) in map {
                sources.insert(id.to_owned(), WorkspaceRelativePathBuf::from(source));
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

        let extended_data = context
            .extend_project_graph
            .emit(ExtendProjectGraphEvent {
                sources: sources.clone(),
                workspace_root: context.workspace_root.to_owned(),
            })
            .await?;

        // Find the root project
        self.root_id = sources.iter().find_map(|(id, source)| {
            if source.as_str().is_empty() || source.as_str() == "." {
                Some(id.to_owned())
            } else {
                None
            }
        });

        dbg!("preload", &sources, &extended_data.aliases);

        self.sources = sources;
        self.aliases = extended_data.aliases;

        Ok(())
    }

    fn context(&self) -> &ProjectGraphBuilderContext<'app> {
        self.context.as_ref().unwrap()
    }

    fn resolve_id(&self, project_locator: &str) -> Id {
        let id = match self.aliases.get(project_locator) {
            Some(project_id) => project_id.to_owned(),
            None => Id::raw(project_locator),
        };

        match self.renamed_ids.get(&id) {
            Some(new_id) => new_id.to_owned(),
            None => id,
        }
    }
}
