use moon_action_graph::*;
use moon_test_utils2::WorkspaceMocker;
use moon_workspace_graph::WorkspaceGraph;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ActionGraphContainer {
    pub mocker: WorkspaceMocker,
}

impl ActionGraphContainer {
    pub fn new(root: &Path) -> Self {
        Self {
            mocker: WorkspaceMocker::new(root)
                .load_default_configs()
                .with_all_toolchains()
                .with_test_toolchains()
                .with_default_projects()
                .with_global_envs(),
        }
    }

    pub fn set_working_dir(mut self, dir: PathBuf) -> Self {
        self.mocker = self.mocker.set_working_dir(dir);
        self
    }

    pub async fn create_workspace_graph(&self) -> Arc<WorkspaceGraph> {
        Arc::new(self.mocker.mock_workspace_graph().await)
    }

    pub async fn create_builder(
        &mut self,
        workspace_graph: Arc<WorkspaceGraph>,
    ) -> ActionGraphBuilder {
        let config = &self.mocker.workspace_config.pipeline;

        self.create_builder_with_options(
            workspace_graph,
            ActionGraphBuilderOptions {
                install_dependencies: config.install_dependencies.clone(),
                sync_projects: config.sync_projects.clone(),
                sync_workspace: config.sync_workspace,
                ..Default::default()
            },
        )
        .await
    }

    pub async fn create_builder_with_options(
        &mut self,
        workspace_graph: Arc<WorkspaceGraph>,
        options: ActionGraphBuilderOptions,
    ) -> ActionGraphBuilder {
        let mut builder = ActionGraphBuilder::new(
            Arc::new(self.mocker.mock_app_context()),
            workspace_graph,
            options,
        )
        .unwrap();
        builder
            .set_platform_manager(self.mocker.mock_platform_manager().await)
            .unwrap();
        builder
    }
}
