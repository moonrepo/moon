use crate::session::MoonSession;
use moon_daemon::AtomicDaemonState;
use tokio::task::JoinHandle;
use tracing::trace;

impl MoonSession {
    pub fn download_extensions(&self) {
        trace!("Downloading extensions");

        let session = self.clone();

        tokio::spawn(async move {
            if let Ok(registry) = session.get_extension_registry().await {
                let _ = registry.load_all().await;
            }
        });
    }

    pub fn download_toolchains(&self) {
        trace!("Downloading toolchains");

        let session = self.clone();

        tokio::spawn(async move {
            if let Ok(registry) = session.get_toolchain_registry().await {
                let _ = registry.load_all().await;
            }
        });
    }

    pub fn rebuild_graphs(&self, state: AtomicDaemonState) -> JoinHandle<()> {
        trace!("Rebuilding project and task graphs");

        let session = self.clone();

        tokio::spawn(async move {
            if let Ok(graph) = session.get_workspace_graph().await {
                state.write().await.workspace_graph = graph;
            }
        })
    }

    pub fn reset_components(&mut self) {
        trace!("Resetting registries and graphs cache");

        self.extension_registry.take();
        self.toolchain_registry.take();
        self.project_graph.take();
        self.task_graph.take();
        self.workspace_graph.take();
    }

    pub fn reset_vcs(&mut self) {
        trace!("Resetting VCS adapter");

        self.vcs_adapter.take();
    }
}
