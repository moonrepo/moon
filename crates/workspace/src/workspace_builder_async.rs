use crate::projects_builder::*;
use crate::tasks_builder::*;
use crate::workspace_builder::*;
use crate::workspace_cache::*;
use moon_common::Id;
use moon_graph_utils::GraphExpanderContext;
use moon_hash::Digest;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use starbase_utils::json;
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilderAsync {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// Builder for everything projects related.
    projects: WorkspaceProjectsBuilder,

    /// Builder for everything tasks related.
    tasks: WorkspaceTasksBuilder,
}

impl WorkspaceBuilderAsync {
    pub async fn new(context: WorkspaceBuilderContext) -> miette::Result<Self> {
        debug!("Building workspace graph asynchronously (project and task graphs)");

        let context = Arc::new(context);

        Ok(WorkspaceBuilderAsync {
            projects: WorkspaceProjectsBuilder::new(Arc::clone(&context)),
            tasks: WorkspaceTasksBuilder::new(),
            context: Some(context),
        })
    }

    #[instrument(skip_all)]
    pub async fn new_with_cache(context: WorkspaceBuilderContext) -> miette::Result<Self> {
        let is_vcs_enabled = context
            .vcs
            .as_ref()
            .expect("VCS is required for workspace graph caching!")
            .is_enabled();
        let mut graph = Self::new(context).await?;

        // No VCS to hash with, so abort caching
        if !is_vcs_enabled {
            graph.load_graphs().await?;

            return Ok(graph);
        }

        // Create a lock to avoid colliding cache writes
        let context = graph.context();
        let _lock = context.cache_engine.create_lock(LOCK_FILE_NAME)?;

        // Preload sources and configs, and hash the graph based on that state
        graph.preload().await?;

        let digest = graph.generate_cache_digest().await?;

        debug!(
            hash = digest.hash.as_str(),
            "Generated hash for workspace graph"
        );

        // Check the current state and cache
        let mut state = context
            .cache_engine
            .state
            .load_state::<WorkspaceGraphCacheState>(STATE_CACHE_FILE_NAME)?;
        let cache_path = context
            .cache_engine
            .state
            .resolve_path(STATE_GRAPH_FILE_NAME);

        if digest.hash == state.data.last_hash && cache_path.exists() {
            let mut cache: WorkspaceBuilderAsync = json::read_file(&cache_path)?;

            // Verify that the cached projects match the current projects
            // on disk. If a project has been added or removed since the
            // cache was created, we need to rebuild the graph
            let cached_ids: FxHashSet<&Id> = cache.projects.ids_to_indexes.keys().collect();
            let current_ids: FxHashSet<&Id> = graph.projects.build_data.keys().collect();

            if cached_ids == current_ids {
                debug!(
                    cache = ?cache_path,
                    "Loading workspace graph with {} projects from cache",
                    cache.projects.ids_to_indexes.len(),
                );

                cache.projects.context = graph.projects.context.take();
                cache.context = graph.context;

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

        state.data.last_hash = digest.hash;
        state.save()?;

        json::write_file(cache_path, &graph, false)?;

        Ok(graph)
    }

    pub async fn preload(&mut self) -> miette::Result<()> {
        self.projects.preload().await?;

        Ok(())
    }

    pub async fn load_graphs(&mut self) -> miette::Result<()> {
        if self.has_loaded_graphs() {
            return Ok(());
        }

        self.projects.build(None).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;

        Ok(())
    }

    pub async fn load_graphs_for(&mut self, ids: Vec<Id>) -> miette::Result<()> {
        if self.has_loaded_graphs() {
            return Ok(());
        }

        self.projects.build(Some(ids)).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;

        Ok(())
    }

    fn has_loaded_graphs(&self) -> bool {
        self.projects.graph.node_count() > 0
    }

    async fn generate_cache_digest(&self) -> miette::Result<Digest> {
        generate_graph_cache_digest(
            self.context(),
            &self.projects.build_data,
            self.projects.config_paths.iter().cloned().collect(),
            true,
        )
        .await
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(self) -> miette::Result<WorkspaceGraph> {
        let context = self.context();

        // Enforce constraints before finalizing, so that they also
        // apply to graphs that were loaded from the cache
        self.projects.enforce_constraints()?;

        let mut graph_context = GraphExpanderContext {
            working_dir: context.working_dir.clone(),
            workspace_root: context.workspace_root.clone(),
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
