use moon_action_graph::ActionGraphBuilder;
use moon_platform::PlatformManager;
use moon_project_graph::ProjectGraph;
use moon_test_utils2::{
    generate_platform_manager_from_sandbox, generate_project_graph_from_sandbox,
};
use std::path::Path;

pub struct ActionGraphContainer {
    pub platform_manager: PlatformManager,
    pub project_graph: ProjectGraph,
}

impl ActionGraphContainer {
    pub async fn new(root: &Path) -> Self {
        Self {
            platform_manager: generate_platform_manager_from_sandbox(root).await,
            project_graph: generate_project_graph_from_sandbox(root).await,
        }
    }

    pub fn create_builder(&self) -> ActionGraphBuilder {
        ActionGraphBuilder::with_platforms(&self.platform_manager, &self.project_graph).unwrap()
    }
}
