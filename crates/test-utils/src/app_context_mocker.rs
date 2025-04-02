use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_config::*;
use moon_console::{Console, MoonReporter};
use moon_env::MoonEnvironment;
use moon_plugin::PluginHostData;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace_graph::WorkspaceGraph;
use proto_core::{ProtoConfig, ProtoEnvironment};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct AppContextMocker {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub moon_env: MoonEnvironment,
    pub proto_env: ProtoEnvironment,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_graph: Option<WorkspaceGraph>,
    pub workspace_root: PathBuf,
}

impl AppContextMocker {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();

        Self {
            moon_env: MoonEnvironment::new_testing(root),
            proto_env: ProtoEnvironment::new_testing(root).unwrap(),
            workspace_root: root.to_path_buf(),
            ..Default::default()
        }
    }

    pub fn load_root_configs(&mut self) -> &mut Self {
        self.inherited_tasks = self
            .config_loader
            .load_tasks_manager(&self.workspace_root)
            .unwrap();

        self.toolchain_config = self
            .config_loader
            .load_toolchain_config(&self.workspace_root, &ProtoConfig::default())
            .unwrap();

        self.workspace_config = self
            .config_loader
            .load_workspace_config(&self.workspace_root)
            .unwrap();

        self
    }

    pub fn with_global_envs(&mut self) -> &mut Self {
        #[allow(deprecated)]
        let home_dir = std::env::home_dir().unwrap();

        self.moon_env = MoonEnvironment::from(home_dir.join(".moon")).unwrap();
        self.moon_env.working_dir = self.workspace_root.clone();
        self.moon_env.workspace_root = self.workspace_root.clone();

        self.proto_env = ProtoEnvironment::from(home_dir.join(".proto"), home_dir).unwrap();
        self.proto_env.working_dir = self.workspace_root.clone();
        self
    }

    pub fn with_workspace_graph(&mut self, graph: WorkspaceGraph) -> &mut Self {
        self.workspace_graph = Some(graph);
        self
    }

    pub fn mock(mut self) -> AppContext {
        self.toolchain_config.inherit_plugin_locators().unwrap();

        AppContext {
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            cache_engine: Arc::new(self.build_cache_engine()),
            console: Arc::new(self.build_console()),
            toolchain_registry: Arc::new(self.build_toolchain_registry()),
            vcs: Arc::new(self.build_vcs()),
            toolchain_config: Arc::new(self.toolchain_config),
            working_dir: self.workspace_root.clone(),
            workspace_config: Arc::new(self.workspace_config),
            workspace_root: self.workspace_root,
        }
    }

    fn build_cache_engine(&self) -> CacheEngine {
        CacheEngine::new(&self.workspace_root).unwrap()
    }

    fn build_console(&self) -> Console {
        let mut console = Console::new_testing();
        console.set_reporter(MoonReporter::default());
        console
    }

    fn build_toolchain_registry(&self) -> ToolchainRegistry {
        let mut registry = ToolchainRegistry::new(PluginHostData {
            moon_env: Arc::new(self.moon_env.clone()),
            proto_env: Arc::new(self.proto_env.clone()),
            workspace_graph: Arc::new(RwLock::new(
                self.workspace_graph.clone().unwrap_or_default(),
            )),
        });
        registry.inherit_configs(&self.toolchain_config.plugins);
        registry
    }

    fn build_vcs(&self) -> BoxedVcs {
        Box::new(
            Git::load(
                &self.workspace_root,
                &self.workspace_config.vcs.default_branch,
                &self.workspace_config.vcs.remote_candidates,
            )
            .unwrap(),
        )
    }
}
