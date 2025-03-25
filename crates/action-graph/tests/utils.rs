use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_config::WorkspaceConfig;
use moon_platform::PlatformManager;
use moon_test_utils2::{
    generate_platform_manager_from_sandbox, generate_workspace_graph_from_sandbox,
};
use moon_workspace_graph::WorkspaceGraph;
use std::path::Path;

pub struct ActionGraphContainer {
    pub platform_manager: PlatformManager,
    pub workspace_graph: WorkspaceGraph,
    pub workspace_config: WorkspaceConfig,
}

impl ActionGraphContainer {
    pub async fn new(root: &Path) -> Self {
        Self {
            platform_manager: generate_platform_manager_from_sandbox(root).await,
            workspace_graph: generate_workspace_graph_from_sandbox(root).await,
            workspace_config: WorkspaceConfig::default(),
        }
    }

    pub fn create_builder(&self) -> ActionGraphBuilder {
        let config = &self.workspace_config.pipeline;

        ActionGraphBuilder::with_platforms(
            &self.platform_manager,
            &self.workspace_graph,
            ActionGraphBuilderOptions {
                install_dependencies: config.install_dependencies.clone(),
                setup_toolchains: true.into(),
                sync_projects: config.sync_projects.clone(),
                sync_workspace: config.sync_workspace,
            },
        )
        .unwrap()
    }
}
