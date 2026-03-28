use crate::session::MoonSession;
use moon_config::{ExtensionsConfig, InheritedTasksManager, ToolchainsConfig, WorkspaceConfig};
use proto_core::ProtoEnvironment;
use std::sync::Arc;
use tokio::sync::Mutex;

impl MoonSession {
    pub fn regenerate_graphs(&self, mutex: Arc<Mutex<()>>) {
        let session = self.clone();

        // If the graph is already being regenerated, skip this
        if mutex.try_lock().is_err() {
            return;
        }

        tokio::spawn(async move {
            // Ensure that multiple threads don't regenerate the graphs at the same time
            let _lock = mutex.lock().await;

            session.get_workspace_graph().await.ok();
        });
    }

    pub fn reset_components(&mut self) {
        self.extension_registry.take();
        self.toolchain_registry.take();
        self.project_graph.take();
        self.task_graph.take();
        self.workspace_graph.take();
    }

    pub fn reset_vcs(&mut self) {
        self.vcs_adapter.take();
    }

    pub fn set_proto_env(&mut self, proto_env: ProtoEnvironment) {
        self.proto_env = Arc::new(proto_env);
        self.reset_components();
    }

    pub fn set_extensions_config(&mut self, extensions_config: ExtensionsConfig) {
        self.extensions_config = Arc::new(extensions_config);
        self.reset_components();
    }

    pub fn set_tasks_config(&mut self, tasks_manager: InheritedTasksManager) {
        self.tasks_config = Arc::new(tasks_manager);
        self.reset_components();
    }

    pub fn set_toolchains_config(&mut self, toolchains_config: ToolchainsConfig) {
        self.toolchains_config = Arc::new(toolchains_config);
        self.reset_components();
    }

    pub fn set_workspace_config(&mut self, workspace_config: WorkspaceConfig) {
        self.workspace_config = Arc::new(workspace_config);
        self.reset_vcs();
        self.reset_components();
    }
}
