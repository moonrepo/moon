use moon_action_graph::{
    ActionGraphBuilder, ActionGraphBuilderOptions,
    action_graph_builder2::{
        ActionGraphBuilder as ActionGraphBuilder2,
        ActionGraphBuilderOptions as ActionGraphBuilderOptions2,
    },
};
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
        let app_context = WorkspaceMocker::new(&self.workspace_root)
            .with_all_toolchains()
            .mock_app_context();

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

pub struct ActionGraphContainer2 {
    pub mocker: WorkspaceMocker,
    pub platform: Option<PlatformManager>,
}

impl ActionGraphContainer2 {
    pub fn new(root: &Path) -> Self {
        Self {
            mocker: WorkspaceMocker::new(root)
                .load_default_configs()
                .with_default_projects()
                .with_global_envs(),
            platform: None,
        }
    }

    pub async fn create_workspace_graph(&self) -> Arc<WorkspaceGraph> {
        Arc::new(self.mocker.mock_workspace_graph().await)
    }

    pub async fn create_builder(
        &mut self,
        workspace_graph: Arc<WorkspaceGraph>,
    ) -> ActionGraphBuilder2 {
        let config = &self.mocker.workspace_config.pipeline;

        self.create_builder_with_options(
            workspace_graph,
            ActionGraphBuilderOptions2 {
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
        options: ActionGraphBuilderOptions2,
    ) -> ActionGraphBuilder2 {
        if self.platform.is_none() {
            self.platform = Some(self.mocker.mock_platform_manager().await);
        }

        // ActionGraphBuilder2::with_platforms(
        // self.platform.as_ref().unwrap(),
        ActionGraphBuilder2::new(
            Arc::new(self.mocker.mock_app_context()),
            workspace_graph,
            options,
        )
        .unwrap()
    }
}
