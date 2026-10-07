use crate::projects_builder::*;
use crate::tasks_builder::*;
use crate::vcs_info::*;
use crate::workspace_cache::*;
use miette::IntoDiagnostic;
use moon_cache::CacheEngine;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::{ExtensionsConfig, InheritedTasksManager, ToolchainsConfig, WorkspaceConfig};
use moon_config_loader::ConfigLoader;
use moon_extension_plugin::ExtensionRegistry;
use moon_graph_utils::{GraphExpanderContext, NodeState};
use moon_hash::{ContentHasher, Digest};
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::BoxedVcs;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, instrument};

pub const LOCK_FILE_NAME: &str = "workspaceGraph.lock";
pub const STATE_GRAPH_FILE_NAME: &str = "workspaceGraph.json";
pub const STATE_CACHE_FILE_NAME: &str = "workspaceGraphStateV1.json";

pub struct WorkspaceBuilderContext {
    pub cache_engine: Arc<CacheEngine>,
    pub config_loader: ConfigLoader,
    pub enabled_toolchains: Vec<Id>,
    pub extensions_config: Arc<ExtensionsConfig>,
    pub extension_registry: Arc<ExtensionRegistry>,
    pub inherited_tasks: Arc<InheritedTasksManager>,
    pub toolchains_config: Arc<ToolchainsConfig>,
    pub toolchain_registry: Arc<ToolchainRegistry>,
    pub vcs: Option<Arc<BoxedVcs>>,
    pub working_dir: PathBuf,
    pub workspace_config: Arc<WorkspaceConfig>,
    pub workspace_root: PathBuf,
}

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilder {
    /// The context is not serialized, so it's optional for deserializing
    /// a cached builder, and is set immediately after.
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// Whether the graphs have been loaded. A cached builder has always
    /// been loaded, and is marked as such immediately after deserializing.
    #[serde(skip)]
    loaded: bool,

    /// VCS information, which is loaded in the background while
    /// the graphs are built, and awaited when finalizing them.
    #[serde(skip)]
    vcs_handle: Option<JoinHandle<miette::Result<VcsInfo>>>,

    /// Builder for everything projects related.
    projects: WorkspaceProjectsBuilder,

    /// Builder for everything tasks related.
    tasks: WorkspaceTasksBuilder,
}

impl WorkspaceBuilder {
    pub async fn new(context: WorkspaceBuilderContext) -> miette::Result<Self> {
        debug!("Building workspace graph (project and task graphs)");

        let context = Arc::new(context);

        Ok(WorkspaceBuilder {
            loaded: false,
            projects: WorkspaceProjectsBuilder::new(Arc::clone(&context)),
            tasks: WorkspaceTasksBuilder::new(),
            vcs_handle: context
                .vcs
                .clone()
                .map(|vcs| tokio::spawn(load_vcs_info(vcs))),
            context: Some(context),
        })
    }

    #[instrument(skip_all)]
    pub async fn new_with_cache(context: WorkspaceBuilderContext) -> miette::Result<Self> {
        let is_vcs_enabled = context.vcs.as_ref().is_some_and(|vcs| vcs.is_enabled());
        let mut graph = Self::new(context).await?;

        // No VCS to hash with, so abort caching
        if !is_vcs_enabled {
            graph.load_graphs().await?;

            return Ok(graph);
        }

        // Create a lock to avoid colliding cache writes
        let context = graph.context();
        let _lock = context.cache_engine.create_lock(LOCK_FILE_NAME)?;

        // Load the previous state, as input files discovered by plugins
        // during the last build must contribute to the hash
        let mut state = context
            .cache_engine
            .state
            .load_state::<WorkspaceGraphCacheState>(STATE_CACHE_FILE_NAME)?;
        let cache_path = context
            .cache_engine
            .state
            .resolve_path(STATE_GRAPH_FILE_NAME);

        // Preload sources and configs, and hash the graph based on that state
        graph.preload().await?;

        // Capture the project sources now, as `load_graphs` consumes the
        // build data, and they're needed if the digest is regenerated after
        let projects = graph
            .projects
            .build_data
            .iter()
            .map(|(id, build_data)| (id.clone(), build_data.source.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut hasher = graph
            .create_cache_hasher(&projects, state.data.plugin_input_paths.clone())
            .await?;
        let digest = Digest::from_hasher(&mut hasher)?;

        debug!(
            hash = digest.hash.as_str(),
            "Generated hash for workspace graph"
        );

        if digest.hash == state.data.last_hash && cache_path.exists() {
            let mut cache: WorkspaceBuilder = read_cache_file(&cache_path)?;

            // Verify that the cached projects match the current projects
            // on disk. If a project has been added or removed since the
            // cache was created, we need to rebuild the graph
            let cached_ids: FxHashSet<&Id> = cache
                .projects
                .graph
                .node_weights()
                .filter_map(|node| match node {
                    NodeState::Loaded(project) => Some(&project.id),
                    NodeState::Loading => None,
                })
                .collect();
            let current_ids: FxHashSet<&Id> = graph.projects.build_data.keys().collect();

            if cached_ids == current_ids {
                debug!(
                    cache = ?cache_path,
                    "Loading workspace graph with {} projects from cache",
                    cached_ids.len(),
                );

                cache.projects.context = graph.projects.context.take();
                cache.context = graph.context;
                cache.loaded = true;
                cache.vcs_handle = graph.vcs_handle;

                context
                    .cache_engine
                    .storage
                    .store_hash_manifest_with_hasher(hasher)
                    .await?;

                return Ok(cache);
            }

            debug!(
                cache = ?cache_path,
                "Cached workspace graph has mismatched projects, rebuilding",
            );
        }

        // Build the graph, update the state, and save the cache
        debug!(
            "Preparing workspace graph with {} projects",
            graph.projects.build_data.len(),
        );

        graph.load_graphs().await?;

        // If plugins discovered a different set of input files, regenerate
        // the hash with them included, otherwise the next run would be
        // a guaranteed cache miss
        if graph.projects.plugin_input_paths != state.data.plugin_input_paths {
            state.data.plugin_input_paths = graph.projects.plugin_input_paths.clone();

            hasher = graph
                .create_cache_hasher(&projects, state.data.plugin_input_paths.clone())
                .await?;
        }

        // Only store the final manifest, as the hash may have been regenerated
        let digest = context
            .cache_engine
            .storage
            .store_hash_manifest_with_hasher(hasher)
            .await?;

        state.data.last_hash = digest.hash;
        state.save()?;

        write_cache_file(&cache_path, &graph)?;

        Ok(graph)
    }

    pub async fn preload(&mut self) -> miette::Result<()> {
        self.projects.preload().await?;

        Ok(())
    }

    pub async fn load_graphs(&mut self) -> miette::Result<()> {
        if self.loaded {
            return Ok(());
        }

        self.projects.build(None).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;
        self.loaded = true;

        Ok(())
    }

    pub async fn load_graphs_for(&mut self, ids: Vec<Id>) -> miette::Result<()> {
        if self.loaded {
            return Ok(());
        }

        self.projects.build(Some(ids)).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;
        self.loaded = true;

        Ok(())
    }

    async fn create_cache_hasher(
        &self,
        projects: &BTreeMap<Id, WorkspaceRelativePathBuf>,
        plugin_input_paths: BTreeSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<ContentHasher> {
        create_graph_cache_hasher(
            self.context(),
            projects,
            self.projects.config_paths.iter().cloned().collect(),
            plugin_input_paths,
        )
        .await
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(mut self) -> miette::Result<WorkspaceGraph> {
        let context = self.context();

        // Enforce constraints before finalizing, so that they also
        // apply to graphs that were loaded from the cache
        self.projects.enforce_constraints()?;

        let mut graph_context = GraphExpanderContext {
            config_dir: context.config_loader.dir.clone(),
            extensions_config: context.extensions_config.clone(),
            inherited_tasks: context.inherited_tasks.clone(),
            toolchains_config: context.toolchains_config.clone(),
            working_dir: context.working_dir.to_owned(),
            workspace_config: context.workspace_config.clone(),
            workspace_root: context.workspace_root.to_owned(),
            ..Default::default()
        };

        if let Some(vcs_handle) = self.vcs_handle.take() {
            let vcs_info = vcs_handle.await.into_diagnostic()??;

            graph_context.vcs_branch = Arc::new(vcs_info.branch);
            graph_context.vcs_repository = Arc::new(vcs_info.repository);
            graph_context.vcs_revision = Arc::new(vcs_info.revision);
        }

        // Build the graphs
        let project_graph = Arc::new(self.projects.finalize(graph_context.clone())?);

        let task_graph = Arc::new(
            self.tasks
                .finalize(graph_context, Arc::clone(&project_graph)),
        );

        Ok(WorkspaceGraph::new(
            project_graph,
            task_graph,
            context.workspace_root.to_path_buf(),
        ))
    }

    pub fn context(&self) -> Arc<WorkspaceBuilderContext> {
        Arc::clone(
            self.context
                .as_ref()
                .expect("Missing workspace builder context!"),
        )
    }
}
