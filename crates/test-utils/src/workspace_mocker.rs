use crate::generate_platform_manager;
use moon_action_graph::ActionGraphBuilder;
use moon_action_pipeline::ActionPipeline;
use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_common::{Id, path::WorkspaceRelativePathBuf};
use moon_config::*;
use moon_console::{Console, MoonReporter};
use moon_env::MoonEnvironment;
use moon_platform::PlatformManager;
use moon_plugin::PluginHostData;
use moon_project_builder::*;
use moon_project_graph::Project;
use moon_task_builder::*;
use moon_task_graph::Task;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace::*;
pub use moon_workspace_graph::WorkspaceGraph;
use proto_core::warpgate::FileLocator;
use proto_core::{PluginLocator, ProtoConfig, ProtoEnvironment};
use starbase_events::Emitter;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Default)]
pub struct WorkspaceMocker {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub monorepo: bool,
    pub moon_env: MoonEnvironment,
    pub proto_env: ProtoEnvironment,
    pub toolchain_config: ToolchainConfig,
    pub working_dir: PathBuf,
    pub workspace_config: WorkspaceConfig,
    pub workspace_root: PathBuf,
}

impl WorkspaceMocker {
    pub fn new(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();

        Self {
            monorepo: true,
            moon_env: MoonEnvironment::new_testing(root),
            proto_env: ProtoEnvironment::new_testing(root).unwrap(),
            working_dir: root.to_path_buf(),
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

    pub fn load_inherited_tasks_from(mut self, dir: &str) -> Self {
        self.inherited_tasks = self
            .config_loader
            .load_tasks_manager_from(&self.workspace_root, self.workspace_root.join(dir))
            .unwrap();

        self
    }

    pub fn set_polyrepo(mut self) -> Self {
        self.monorepo = false;
        self
    }

    pub fn set_toolchain_config(mut self, mut config: ToolchainConfig) -> Self {
        config.inherit_plugin_locators().unwrap();

        self.toolchain_config = config;
        self
    }

    pub fn set_working_dir(mut self, dir: PathBuf) -> Self {
        self.moon_env.working_dir = dir.clone();
        self.proto_env.working_dir = dir.clone();
        self.working_dir = dir;
        self
    }

    pub fn set_workspace_config(mut self, config: WorkspaceConfig) -> Self {
        self.workspace_config = config;
        self
    }

    pub fn update_toolchain_config(mut self, mut op: impl FnMut(&mut ToolchainConfig)) -> Self {
        op(&mut self.toolchain_config);
        self.toolchain_config.inherit_plugin_locators().unwrap();
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

    pub fn with_all_toolchains(self) -> Self {
        self.update_toolchain_config(|config| {
            config.bun = Some(BunConfig::default());
            config.deno = Some(DenoConfig::default());
            config.node = Some(NodeConfig::default());
            config.rust = Some(RustConfig::default());
            config.inherit_default_plugins().unwrap();
        })
    }

    pub fn with_test_toolchains(self) -> Self {
        let target_dir = match std::env::var("CARGO_TARGET_DIR") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => {
                let start_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                let mut current_dir = Some(start_dir.as_path());

                while let Some(dir) = current_dir {
                    if dir.join("Cargo.lock").exists() {
                        break;
                    }

                    match dir.parent() {
                        Some(parent) => current_dir = Some(parent),
                        None => {
                            panic!("Unable to find the Cargo workspace root!");
                        }
                    }
                }

                current_dir.unwrap().join("wasm").join("target")
            }
        };

        self.update_toolchain_config(|config| {
            for id in ["tc-tier1", "tc-tier2", "tc-tier2-setup-env", "tc-tier3"] {
                config.plugins.insert(
                    Id::raw(id),
                    ToolchainPluginConfig {
                        plugin: Some(PluginLocator::File(Box::new(FileLocator {
                            file: "".into(),
                            path: Some(
                                target_dir
                                    .join("wasm32-wasip1")
                                    .join("release")
                                    .join(format!("{}.wasm", id.replace("-", "_"))),
                            ),
                        }))),
                        version: if id == "tc-tier3" {
                            Some(UnresolvedVersionSpec::parse("1.2.3").unwrap())
                        } else {
                            None
                        },
                        ..Default::default()
                    },
                );
            }
        })
    }

    pub fn with_default_toolchains(self) -> Self {
        self.update_toolchain_config(|config| {
            if config.node.is_none() {
                config.node = Some(NodeConfig::default());
            }
        })
    }

    pub fn with_global_envs(mut self) -> Self {
        #[allow(deprecated)]
        let home_dir = std::env::home_dir().unwrap();

        self.moon_env = MoonEnvironment::from(home_dir.join(".moon")).unwrap();
        self.moon_env.working_dir = self.working_dir.clone();
        self.moon_env.workspace_root = self.workspace_root.clone();
        self.moon_env.test_only = true;

        self.proto_env = ProtoEnvironment::from(home_dir.join(".proto"), home_dir).unwrap();
        self.proto_env.working_dir = self.working_dir.clone();
        self.proto_env.test_only = true;

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

    pub async fn build_project(&self, id: &str) -> Project {
        self.build_project_with(id, |_| {}).await
    }

    pub async fn build_project_with(
        &self,
        id: &str,
        mut op: impl FnMut(&mut ProjectBuilder),
    ) -> Project {
        let source = if id == "root" {
            WorkspaceRelativePathBuf::new()
        } else {
            WorkspaceRelativePathBuf::from(id)
        };
        let id = Id::raw(id);

        let enabled_toolchains = self.toolchain_config.get_enabled();

        let mut builder = ProjectBuilder::new(
            &id,
            &source,
            ProjectBuilderContext {
                config_loader: &self.config_loader,
                enabled_toolchains: &enabled_toolchains,
                monorepo: self.monorepo,
                root_project_id: None,
                toolchain_config: &self.toolchain_config,
                toolchain_registry: Arc::new(self.mock_toolchain_registry()),
                workspace_root: &self.workspace_root,
            },
        )
        .unwrap();

        builder.load_local_config().await.unwrap();
        builder
            .inherit_global_config(&self.inherited_tasks)
            .unwrap();

        op(&mut builder);

        builder.build().await.unwrap()
    }

    pub async fn build_tasks(&self, project: &Project) -> BTreeMap<Id, Task> {
        self.build_tasks_with(project, |_| {}).await
    }

    pub async fn build_tasks_with(
        &self,
        project: &Project,
        mut op: impl FnMut(&mut TasksBuilder),
    ) -> BTreeMap<Id, Task> {
        let toolchain_registry = self.mock_toolchain_registry();
        let enabled_toolchains = self.toolchain_config.get_enabled();

        let mut builder = TasksBuilder::new(
            &project.id,
            &project.source,
            &project.toolchains,
            TasksBuilderContext {
                enabled_toolchains: &enabled_toolchains,
                monorepo: self.monorepo,
                toolchain_config: &self.toolchain_config,
                toolchain_registry: toolchain_registry.into(),
                workspace_root: &self.workspace_root,
            },
        );

        builder.load_local_tasks(&project.config);

        let global_config = self
            .inherited_tasks
            .get_inherited_config(
                &project.toolchains,
                &project.config.stack,
                &project.config.layer,
                &project.config.tags,
            )
            .unwrap();

        builder.inherit_global_tasks(
            &global_config.config,
            Some(&project.config.workspace.inherited_tasks),
        );

        op(&mut builder);

        builder.build().await.unwrap()
    }

    pub async fn create_action_graph(&self) -> ActionGraphBuilder {
        ActionGraphBuilder::new(
            self.mock_app_context().into(),
            self.mock_workspace_graph().await.into(),
            Default::default(),
        )
        .unwrap()
    }

    pub async fn mock_action_pipeline(&self) -> ActionPipeline {
        ActionPipeline::new(
            self.mock_app_context().into(),
            self.mock_workspace_graph().await.into(),
        )
    }

    pub fn mock_app_context(&self) -> AppContext {
        AppContext {
            cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
            cache_engine: Arc::new(self.mock_cache_engine()),
            console: Arc::new(self.mock_console()),
            toolchain_registry: Arc::new(self.mock_toolchain_registry()),
            vcs: Arc::new(self.mock_vcs_adapter()),
            toolchain_config: Arc::new(self.toolchain_config.clone()),
            working_dir: self.working_dir.clone(),
            workspace_config: Arc::new(self.workspace_config.clone()),
            workspace_root: self.workspace_root.clone(),
        }
    }

    pub fn mock_cache_engine(&self) -> CacheEngine {
        CacheEngine::new(&self.workspace_root).unwrap()
    }

    pub fn mock_console(&self) -> Console {
        let mut console = Console::new_testing();
        console.set_reporter(MoonReporter::new_testing());
        console
    }

    pub async fn mock_platform_manager(&self) -> PlatformManager {
        generate_platform_manager(
            &self.workspace_root,
            &self.toolchain_config,
            Arc::new(self.proto_env.clone()),
            Arc::new(self.mock_console()),
        )
        .await
    }

    pub fn mock_toolchain_registry(&self) -> ToolchainRegistry {
        let mut registry = ToolchainRegistry::new(PluginHostData {
            moon_env: Arc::new(self.moon_env.clone()),
            proto_env: Arc::new(self.proto_env.clone()),
            workspace_graph: Arc::new(OnceLock::new()),
        });
        registry.inherit_configs(&self.toolchain_config.plugins);
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
            working_dir: &self.working_dir,
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
