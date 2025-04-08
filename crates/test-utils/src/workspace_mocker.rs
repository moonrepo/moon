use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_config::*;
use moon_console::{Console, MoonReporter};
use moon_env::MoonEnvironment;
use moon_plugin::PluginHostData;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace::*;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::{ProtoConfig, ProtoEnvironment};
use starbase_events::Emitter;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Default)]
pub struct WorkspaceMocker {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub moon_env: MoonEnvironment,
    pub proto_env: ProtoEnvironment,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
    pub vcs: Option<BoxedVcs>,
}

impl WorkspaceMocker {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();

        Self {
            moon_env: MoonEnvironment::new_testing(root),
            proto_env: ProtoEnvironment::new_testing(root).unwrap(),
            workspace_root: root.to_path_buf(),
            ..Default::default()
        }
    }

    pub fn load_default_configs(mut self) -> Self {
        let root = &self.workspace_root;

        self.inherited_tasks = self.config_loader.load_tasks_manager(root).unwrap();

        self.toolchain_config = self
            .config_loader
            .load_toolchain_config(root, &ProtoConfig::default())
            .unwrap();

        self.workspace_config = self.config_loader.load_workspace_config(root).unwrap();

        self
    }

    pub fn set_toolchain_config(mut self, config: ToolchainConfig) -> Self {
        self.toolchain_config = config;
        self
    }

    pub fn set_workspace_config(mut self, config: WorkspaceConfig) -> Self {
        self.workspace_config = config;
        self
    }

    pub fn update_toolchain_config(mut self, mut op: impl FnMut(&mut ToolchainConfig)) -> Self {
        op(&mut self.toolchain_config);
        self
    }

    pub fn update_workspace_config(mut self, mut op: impl FnMut(&mut WorkspaceConfig)) -> Self {
        op(&mut self.workspace_config);
        self
    }

    pub fn with_default_projects(mut self) -> Self {
        if !self.workspace_root.join(".moon/workspace.yml").exists() {
            // Use folders as project names
            let mut projects = WorkspaceProjectsConfig {
                globs: vec![
                    "*".into(),
                    "!.home".into(),
                    "!.moon".into(),
                    "!.proto".into(),
                ],
                ..WorkspaceProjectsConfig::default()
            };

            // Include a root project conditionally
            if self.workspace_root.join("moon.yml").exists() {
                projects
                    .sources
                    .insert("root".try_into().unwrap(), ".".into());
            }

            self.workspace_config.projects = WorkspaceProjects::Both(projects);
        }

        self
    }

    pub fn with_default_toolchains(mut self) -> Self {
        if self.toolchain_config.node.is_none() {
            self.toolchain_config.node = Some(NodeConfig::default());
        }

        self
    }

    pub fn with_global_envs(mut self) -> Self {
        #[allow(deprecated)]
        let home_dir = std::env::home_dir().unwrap();

        self.moon_env = MoonEnvironment::from(home_dir.join(".moon")).unwrap();
        self.moon_env.working_dir = self.workspace_root.clone();
        self.moon_env.workspace_root = self.workspace_root.clone();

        self.proto_env = ProtoEnvironment::from(home_dir.join(".proto"), home_dir).unwrap();
        self.proto_env.working_dir = self.workspace_root.clone();

        self
    }

    pub fn with_inherited_tasks(mut self) -> Self {
        self.inherited_tasks.configs.insert(
            "*".into(),
            InheritedTasksEntry {
                input: ".moon/tasks.yml".into(),
                config: PartialInheritedTasksConfig {
                    tasks: Some(BTreeMap::from_iter([(
                        "global".try_into().unwrap(),
                        PartialTaskConfig::default(),
                    )])),
                    ..PartialInheritedTasksConfig::default()
                },
            },
        );

        self
    }

    pub fn mock_app_context(&self) -> AppContext {
        let toolchain_config = self.mock_toolchain_config();
        let workspace_config = self.mock_workspace_config();

        AppContext {
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            cache_engine: Arc::new(self.mock_cache_engine()),
            console: Arc::new(self.mock_console()),
            toolchain_registry: Arc::new(self.mock_toolchain_registry()),
            vcs: Arc::new(self.mock_vcs_adapter()),
            toolchain_config: Arc::new(toolchain_config),
            working_dir: self.workspace_root.clone(),
            workspace_config: Arc::new(workspace_config),
            workspace_root: self.workspace_root.clone(),
        }
    }

    pub fn mock_cache_engine(&self) -> CacheEngine {
        CacheEngine::new(&self.workspace_root).unwrap()
    }

    pub fn mock_console(&self) -> Console {
        let mut console = Console::new_testing();
        console.set_reporter(MoonReporter::default());
        console
    }

    pub fn mock_toolchain_config(&self) -> ToolchainConfig {
        let mut config = self.toolchain_config.clone();
        // config.inherit_default_plugins().unwrap();
        config.inherit_plugin_locators().unwrap();
        config
    }

    pub fn mock_toolchain_registry(&self) -> ToolchainRegistry {
        let config = self.mock_toolchain_config();
        let mut registry = ToolchainRegistry::new(PluginHostData {
            moon_env: Arc::new(self.moon_env.clone()),
            proto_env: Arc::new(self.proto_env.clone()),
            workspace_graph: Arc::new(OnceLock::new()),
        });
        registry.inherit_configs(&config.plugins);
        registry
    }

    pub fn mock_vcs_adapter(&self) -> BoxedVcs {
        Box::new(
            Git::load(
                &self.workspace_root,
                &self.workspace_config.vcs.default_branch,
                &self.workspace_config.vcs.remote_candidates,
            )
            .unwrap(),
        )
    }

    pub fn mock_workspace_config(&self) -> WorkspaceConfig {
        self.workspace_config.clone()
    }

    pub fn mock_workspace_builder_context(&self) -> WorkspaceBuilderContext {
        WorkspaceBuilderContext {
            config_loader: &self.config_loader,
            enabled_toolchains: self.toolchain_config.get_enabled(),
            extend_project: Emitter::<ExtendProjectEvent>::new(),
            extend_project_graph: Emitter::<ExtendProjectGraphEvent>::new(),
            inherited_tasks: &self.inherited_tasks,
            toolchain_config: &self.toolchain_config,
            toolchain_registry: Arc::new(self.mock_toolchain_registry()),
            vcs: if self.workspace_root.join(".git").exists() {
                Some(Arc::new(self.mock_vcs_adapter()))
            } else {
                None
            },
            working_dir: &self.workspace_root,
            workspace_config: &self.workspace_config,
            workspace_root: &self.workspace_root,
        }
    }

    pub async fn mock_workspace_graph(&self) -> WorkspaceGraph {
        self.mock_workspace_graph_with_options(WorkspaceMockOptions::default())
            .await
    }

    pub async fn mock_workspace_graph_for(&self, ids: &[&str]) -> WorkspaceGraph {
        self.mock_workspace_graph_with_options(WorkspaceMockOptions {
            ids: Vec::from_iter(ids.iter().map(|id| id.to_string())),
            ..Default::default()
        })
        .await
    }

    pub async fn mock_workspace_graph_with_options(
        &self,
        mut options: WorkspaceMockOptions<'_>,
    ) -> WorkspaceGraph {
        let context = options
            .context
            .take()
            .unwrap_or_else(|| self.mock_workspace_builder_context());

        let mut builder = match &options.cache {
            Some(engine) => WorkspaceBuilder::new_with_cache(context, engine)
                .await
                .unwrap(),
            None => WorkspaceBuilder::new(context).await.unwrap(),
        };

        if options.ids.is_empty() {
            builder.load_projects().await.unwrap();
        } else {
            for id in &options.ids {
                builder.load_project(id).await.unwrap();
            }
        }

        builder.load_tasks().await.unwrap();

        let workspace_graph = builder.build().await.unwrap();

        if options.ids.is_empty() {
            workspace_graph.projects.get_all().unwrap();
        } else {
            for id in &options.ids {
                workspace_graph.projects.get(id).unwrap();
            }
        }

        workspace_graph
    }
}

#[derive(Default)]
pub struct WorkspaceMockOptions<'l> {
    pub cache: Option<CacheEngine>,
    pub context: Option<WorkspaceBuilderContext<'l>>,
    pub ids: Vec<String>,
}
