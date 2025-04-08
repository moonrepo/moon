use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_config::WorkspaceConfig;
use moon_platform::PlatformManager;
use moon_test_utils2::{
    WorkspaceMocker, generate_platform_manager_from_sandbox, generate_workspace_graph_from_sandbox,
};
use moon_workspace_graph::WorkspaceGraph;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ActionGraphContainer {
    pub platform_manager: PlatformManager,
    pub workspace_graph: WorkspaceGraph,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
}

impl ActionGraphContainer {
    pub async fn new(root: &Path) -> Self {
        Self {
            platform_manager: generate_platform_manager_from_sandbox(root).await,
            workspace_graph: generate_workspace_graph_from_sandbox(root).await,
            workspace_config: WorkspaceConfig::default(),
            workspace_root: root.to_path_buf(),
        }
    }

    pub fn create_builder(&self) -> ActionGraphBuilder {
        let config = &self.workspace_config.pipeline;
        let app_context = WorkspaceMocker::new(&self.workspace_root).mock_app_context();

        ActionGraphBuilder::with_platforms(
            &self.platform_manager,
            Arc::new(app_context),
            Arc::new(self.workspace_graph.clone()),
            ActionGraphBuilderOptions {
                install_dependencies: config.install_dependencies.clone(),
                sync_projects: config.sync_projects.clone(),
                sync_workspace: config.sync_workspace,
                ..Default::default()
            },
        )
        .unwrap()
    }
}
