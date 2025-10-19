use moon_action_graph::ActionGraphBuilder;
use moon_action_pipeline::ActionPipeline;
use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_common::{Id, IdExt, path::WorkspaceRelativePathBuf};
use moon_config::*;
use moon_console::{Console, MoonReporter};
use moon_env::MoonEnvironment;
use moon_extension_plugin::ExtensionRegistry;
use moon_plugin::MoonHostData;
use moon_project_builder::*;
use moon_project_graph::Project;
use moon_task_builder::*;
use moon_task_graph::Task;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::{BoxedVcs, git::Git};
use moon_workspace::*;
pub use moon_workspace_graph::WorkspaceGraph;
use proto_core::{ProtoConfig, ProtoEnvironment, warpgate::find_debug_locator};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Default)]
pub struct WorkspaceMocker {
    pub config_loader: ConfigLoader,
    pub inherited_tasks: InheritedTasksManager,
    pub monorepo: bool,
    pub moon_env: MoonEnvironment,
    pub proto_env: ProtoEnvironment,
    pub extensions_config: ExtensionsConfig,
    pub toolchains_config: ToolchainsConfig,
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
            extensions_config: {
                let mut config = ExtensionsConfig::default();
                config.inherit_default_plugins();
                config
            },
            toolchains_config: {
                let mut config = ToolchainsConfig::default();
                config.inherit_system_plugin();
                config.inherit_plugin_locators().unwrap();
                config
            },
            ..Default::default()
        }
    }

    pub fn load_default_configs(mut self) -> Self {
        let root = &self.workspace_root;

        self.inherited_tasks = self.config_loader.load_tasks_manager(root).unwrap();

        self.extensions_config = self.config_loader.load_extensions_config(root).unwrap();

        self.toolchains_config = self
            .config_loader
            .load_toolchains_config(root, &ProtoConfig::default())
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

    pub fn set_toolchains_config(mut self, config: ToolchainsConfig) -> Self {
        self.toolchains_config = config;
        self.update_toolchains_config(|_| {})
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

    pub fn update_extensions_config(mut self, mut op: impl FnMut(&mut ExtensionsConfig)) -> Self {
        op(&mut self.extensions_config);
        self.extensions_config.inherit_default_plugins();
        self
    }

    pub fn update_toolchains_config(mut self, mut op: impl FnMut(&mut ToolchainsConfig)) -> Self {
        op(&mut self.toolchains_config);
        self.toolchains_config.inherit_system_plugin();
        self.toolchains_config.inherit_plugin_locators().unwrap();
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
        self.update_toolchains_config(|config| {
            config.inherit_default_plugins().unwrap();
        })
    }

    pub fn with_test_toolchains(self) -> Self {
        self.update_toolchains_config(|config| {
            for id in [
                "tc-tier1",
                "tc-tier2",
                "tc-tier2-reqs",
                "tc-tier2-setup-env",
                "tc-tier3",
                "tc-tier3-reqs",
            ] {
                let file_name = id.replace("-", "_");

                config.plugins.insert(
                    Id::raw(id),
                    ToolchainPluginConfig {
                        plugin: Some(
                            find_debug_locator(&file_name).expect(
                                "Development plugins missing, build with `just build-wasm`!",
                            ),
                        ),
                        version: if id.contains("tc-tier3") {
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

    pub fn with_global_envs(mut self) -> Self {
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

        let enabled_toolchains = self.toolchains_config.get_enabled();

        let mut builder = ProjectBuilder::new(
            &id,
            &source,
            ProjectBuilderContext {
                config_loader: &self.config_loader,
                enabled_toolchains: &enabled_toolchains,
                monorepo: self.monorepo,
                root_project_id: None,
                toolchains_config: &self.toolchains_config,
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

        let mut enabled_toolchains = self.toolchains_config.get_enabled();
        enabled_toolchains.push(Id::raw("local"));
        enabled_toolchains.push(Id::raw("global"));

        let mut builder = TasksBuilder::new(
            &project.id,
            &project.dependencies,
            &project.source,
            &project.toolchains,
            TasksBuilderContext {
                enabled_toolchains: &enabled_toolchains,
                monorepo: self.monorepo,
                toolchains_config: &self.toolchains_config,
                toolchain_registry: toolchain_registry.into(),
                workspace_root: &self.workspace_root,
            },
        );

        // Note: this list isn't accurate for a real world scenario!
        let stable_toolchains = project
            .toolchains
            .iter()
            .map(Id::stable)
            .collect::<Vec<_>>();

        let global_config = self
            .inherited_tasks
            .get_inherited_config(
                &stable_toolchains,
                &project.config.stack,
                &project.config.layer,
                &project.config.tags,
            )
            .unwrap();

        builder.inherit_global_tasks(
            &global_config.config,
            Some(&project.config.workspace.inherited_tasks),
        );

        builder.load_local_tasks(&project.config);

        op(&mut builder);

        builder.build().await.unwrap()
    }

    pub async fn create_action_graph(&self) -> ActionGraphBuilder<'_> {
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
            moon_env: Arc::new(self.moon_env.clone()),
            proto_env: Arc::new(self.proto_env.clone()),
            extensions_config: Arc::new(self.extensions_config.clone()),
            extension_registry: Arc::new(self.mock_extension_registry()),
            toolchains_config: Arc::new(self.toolchains_config.clone()),
            toolchain_registry: Arc::new(self.mock_toolchain_registry()),
            vcs: Arc::new(self.mock_vcs_adapter()),
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

    pub fn mock_extension_registry(&self) -> ExtensionRegistry {
        let mut registry = ExtensionRegistry::new(MoonHostData {
            moon_env: Arc::new(self.moon_env.clone()),
            proto_env: Arc::new(self.proto_env.clone()),
            extensions_config: Arc::new(self.extensions_config.clone()),
            toolchains_config: Arc::new(self.toolchains_config.clone()),
            workspace_config: Arc::new(self.workspace_config.clone()),
            workspace_graph: Arc::new(OnceLock::new()),
        });
        registry.inherit_configs(&self.extensions_config.plugins);
        registry
    }

    pub fn mock_toolchain_registry(&self) -> ToolchainRegistry {
        let mut registry = ToolchainRegistry::new(
            MoonHostData {
                moon_env: Arc::new(self.moon_env.clone()),
                proto_env: Arc::new(self.proto_env.clone()),
                extensions_config: Arc::new(self.extensions_config.clone()),
                toolchains_config: Arc::new(self.toolchains_config.clone()),
                workspace_config: Arc::new(self.workspace_config.clone()),
                workspace_graph: Arc::new(OnceLock::new()),
            },
            Arc::new(self.toolchains_config.clone()),
        );
        registry.inherit_configs(&self.toolchains_config.plugins);
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

    pub fn mock_workspace_builder_context(&self) -> WorkspaceBuilderContext<'_> {
        WorkspaceBuilderContext {
            config_loader: &self.config_loader,
            enabled_toolchains: self.toolchains_config.get_enabled(),
            inherited_tasks: &self.inherited_tasks,
            toolchains_config: &self.toolchains_config,
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
