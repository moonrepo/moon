use crate::projects_builder::*;
use crate::tasks_builder::*;
use crate::workspace_builder::*;
use moon_cache::CacheEngine;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_graph_utils::GraphExpanderContext;
use moon_workspace_graph::WorkspaceGraph;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, instrument};

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilderAsync {
    #[serde(skip)]
    context: Option<Arc<WorkspaceBuilderContext>>,

    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: FxHashSet<WorkspaceRelativePathBuf>,

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
            config_paths: FxHashSet::default(),
            projects: WorkspaceProjectsBuilder::new(Arc::clone(&context)),
            tasks: WorkspaceTasksBuilder::new(),
            context: Some(context),
        })
    }

    #[instrument(skip_all)]
    pub async fn new_with_cache(
        context: WorkspaceBuilderContext,
        _cache_engine: &CacheEngine,
    ) -> miette::Result<Self> {
        let mut graph = Self::new(context).await?;
        graph.load_graphs().await?;

        Ok(graph)
    }

    pub async fn preload(&mut self) -> miette::Result<()> {
        self.projects.preload().await?;

        Ok(())
    }

    pub async fn load_graphs(&mut self) -> miette::Result<()> {
        self.projects.build(None).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;

        Ok(())
    }

    pub async fn load_graphs_for(&mut self, ids: Vec<Id>) -> miette::Result<()> {
        self.projects.build(Some(ids)).await?;
        self.tasks.build(self.projects.extract_tasks()?)?;

        Ok(())
    }

    /// Build the project graph and return a new structure.
    #[instrument(name = "build_workspace_graph", skip_all)]
    pub async fn build(mut self) -> miette::Result<WorkspaceGraph> {
        let context = self
            .context
            .take()
            .expect("Missing workspace graph builder context!");

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
        let project_graph = Arc::new(self.projects.finalize(graph_context.clone()));

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
}
