use moon_action_graph::ActionGraphBuilder;
use moon_platform::PlatformManager;
use moon_test_utils2::{
    generate_platform_manager_from_sandbox, generate_workspace_graph_from_sandbox,
};
use moon_workspace_graph::WorkspaceGraph;
use std::path::Path;

pub struct ActionGraphContainer {
    pub platform_manager: PlatformManager,
    pub workspace_graph: WorkspaceGraph,
}

impl ActionGraphContainer {
    pub async fn new(root: &Path) -> Self {
        Self {
            platform_manager: generate_platform_manager_from_sandbox(root).await,
            workspace_graph: generate_workspace_graph_from_sandbox(root).await,
        }
    }

    pub fn create_builder(&self) -> ActionGraphBuilder {
        ActionGraphBuilder::with_platforms(&self.platform_manager, &self.workspace_graph).unwrap()
    }
}
