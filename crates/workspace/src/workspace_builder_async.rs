use crate::projects_builder::*;
use crate::tasks_builder::*;
use crate::workspace_builder::*;
use moon_common::path::WorkspaceRelativePathBuf;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

#[derive(Deserialize, Serialize)]
pub struct WorkspaceBuilderAsync {
    /// List of config paths used in the hashing process.
    /// These are used for invalidation.
    config_paths: FxHashSet<WorkspaceRelativePathBuf>,

    /// Builder for everything projects related.
    projects: WorkspaceProjectsBuilder,

    /// Builder for everything tasks related.
    tasks: WorkspaceTasksBuilder,
}

impl WorkspaceBuilderAsync {
    pub async fn new(context: WorkspaceBuilderContext) -> miette::Result<WorkspaceBuilderAsync> {
        debug!("Building workspace graph (project and task graphs)");

        let context = Arc::new(context);

        Ok(WorkspaceBuilderAsync {
            config_paths: FxHashSet::default(),
            projects: WorkspaceProjectsBuilder::new(Arc::clone(&context)),
            tasks: WorkspaceTasksBuilder::new(Arc::clone(&context)),
        })
    }

    pub async fn build_project_graph(&mut self) -> miette::Result<()> {
        self.projects.build().await?;

        Ok(())
    }

    pub async fn build_task_graph(&mut self) -> miette::Result<()> {
        self.tasks.build(&mut self.projects).await?;

        Ok(())
    }
}
