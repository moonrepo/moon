use crate::app::{Cli, Commands};
use crate::app_error::AppError;
use crate::components::*;
use crate::systems::*;
use async_trait::async_trait;
use moon_action_graph::ActionGraphBuilder;
use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_common::{is_ci, is_test_env};
use moon_config::{ConfigLoader, InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_console_reporter::DefaultReporter;
use moon_env::MoonEnvironment;
use moon_extension_plugin::*;
use moon_plugin::{PluginHostData, PluginId};
use moon_project_graph::ProjectGraph;
use moon_task_graph::TaskGraph;
use moon_toolchain_plugin::*;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace::WorkspaceBuilder;
use moon_workspace_graph::WorkspaceGraph;
use once_cell::sync::OnceCell;
use proto_core::ProtoEnvironment;
use semver::Version;
use starbase::{AppResult, AppSession};
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::try_join;
use tracing::debug;

#[derive(Clone)]
pub struct CliSession {
    pub cli: Cli,
    pub cli_version: Version,

    // Components
    pub config_loader: ConfigLoader,
    pub console: Console,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,

    // Lazy components
    cache_engine: OnceCell<Arc<CacheEngine>>,
    extension_registry: OnceCell<Arc<ExtensionRegistry>>,
    project_graph: OnceCell<Arc<ProjectGraph>>,
    task_graph: OnceCell<Arc<TaskGraph>>,
    toolchain_registry: OnceCell<Arc<ToolchainRegistry>>,
    vcs_adapter: OnceCell<Arc<BoxedVcs>>,

    // Configs
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl CliSession {
    pub fn new(cli: Cli, cli_version: String) -> Self {
        debug!("Creating new application session");

        Self {
            cache_engine: OnceCell::new(),
            cli_version: Version::parse(&cli_version).unwrap(),
            config_loader: ConfigLoader::default(),
            console: Console::new(cli.quiet),
            extension_registry: OnceCell::new(),
            moon_env: Arc::new(MoonEnvironment::default()),
            project_graph: OnceCell::new(),
            proto_env: Arc::new(ProtoEnvironment::default()),
            task_graph: OnceCell::new(),
            tasks_config: Arc::new(InheritedTasksManager::default()),
            toolchain_config: Arc::new(ToolchainConfig::default()),
            toolchain_registry: OnceCell::new(),
            working_dir: PathBuf::new(),
            workspace_root: PathBuf::new(),
            workspace_config: Arc::new(WorkspaceConfig::default()),
            vcs_adapter: OnceCell::new(),
            cli,
        }
    }

    pub async fn build_action_graph<'graph>(
        &self,
        workspace_graph: &'graph WorkspaceGraph,
    ) -> miette::Result<ActionGraphBuilder<'graph>> {
        ActionGraphBuilder::new(workspace_graph)
    }

    pub fn get_app_context(&self) -> miette::Result<Arc<AppContext>> {
        Ok(Arc::new(AppContext {
            cli_version: self.cli_version.clone(),
            cache_engine: self.get_cache_engine()?,
            console: Arc::new(self.console.clone()),
            vcs: self.get_vcs_adapter()?,
            toolchain_config: Arc::clone(&self.toolchain_config),
            workspace_config: Arc::clone(&self.workspace_config),
            working_dir: self.working_dir.clone(),
            workspace_root: self.workspace_root.clone(),
        }))
    }

    pub fn get_cache_engine(&self) -> miette::Result<Arc<CacheEngine>> {
        let item = self
            .cache_engine
            .get_or_try_init(|| CacheEngine::new(&self.workspace_root).map(Arc::new))?;

        Ok(Arc::clone(item))
    }

    pub fn get_console(&self) -> miette::Result<Arc<Console>> {
        Ok(Arc::new(self.console.clone()))
    }

    pub async fn get_extension_registry(&self) -> miette::Result<Arc<ExtensionRegistry>> {
        let workspace_graph = self.get_workspace_graph().await?;

        let item = self.extension_registry.get_or_init(|| {
            let mut registry = ExtensionRegistry::new(PluginHostData {
                moon_env: Arc::clone(&self.moon_env),
                proto_env: Arc::clone(&self.proto_env),
                workspace_graph,
            });

            // Convert moon IDs to plugin IDs
            for (id, config) in self.workspace_config.extensions.clone() {
                registry.configs.insert(PluginId::raw(id), config);
            }

            Arc::new(registry)
        });

        Ok(Arc::clone(item))
    }

    pub async fn get_project_graph(&self) -> miette::Result<Arc<ProjectGraph>> {
        if self.project_graph.get().is_none() {
            self.load_workspace_graph().await?;
        }

        Ok(self.project_graph.get().map(Arc::clone).unwrap())
    }

    pub async fn get_task_graph(&self) -> miette::Result<Arc<TaskGraph>> {
        if self.task_graph.get().is_none() {
            self.load_workspace_graph().await?;
        }

        Ok(self.task_graph.get().map(Arc::clone).unwrap())
    }

    pub async fn get_toolchain_registry(&self) -> miette::Result<Arc<ToolchainRegistry>> {
        let workspace_graph = self.get_workspace_graph().await?;

        let item = self.toolchain_registry.get_or_init(|| {
            let mut registry = ToolchainRegistry::new(PluginHostData {
                moon_env: Arc::clone(&self.moon_env),
                proto_env: Arc::clone(&self.proto_env),
                workspace_graph,
            });

            // Convert moon IDs to plugin IDs
            for (id, config) in self.toolchain_config.toolchains.clone() {
                registry.configs.insert(PluginId::raw(id), config);
            }

            Arc::new(registry)
        });

        Ok(Arc::clone(item))
    }

    pub fn get_vcs_adapter(&self) -> miette::Result<Arc<BoxedVcs>> {
        let item = self.vcs_adapter.get_or_try_init(|| {
            let config = &self.workspace_config.vcs;
            let git = Git::load(
                &self.workspace_root,
                &config.default_branch,
                &config.remote_candidates,
            )?;

            Ok::<_, miette::Report>(Arc::new(Box::new(git)))
        })?;

        Ok(Arc::clone(item))
    }

    pub async fn get_workspace_graph(&self) -> miette::Result<WorkspaceGraph> {
        let projects = self.get_project_graph().await?;
        let tasks = self.get_task_graph().await?;

        Ok(WorkspaceGraph::new(projects, tasks))
    }

    pub fn is_telemetry_enabled(&self) -> bool {
        self.workspace_config.telemetry
    }

    pub fn requires_workspace_setup(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Completions(_) | Commands::Init(_) | Commands::Setup
        )
    }

    pub fn requires_toolchain_installed(&self) -> bool {
        matches!(
            self.cli.command,
            Commands::Bin(_) | Commands::Docker { .. } | Commands::Node { .. } | Commands::Teardown
        )
    }

    async fn load_workspace_graph(&self) -> miette::Result<()> {
        let cache_engine = self.get_cache_engine()?;
        let context = create_workspace_graph_context(self).await?;
        let builder = WorkspaceBuilder::new_with_cache(context, &cache_engine).await?;
        let result = builder.build().await?;

        let _ = self.project_graph.set(result.projects);
        let _ = self.task_graph.set(result.tasks);

        Ok(())
    }
}

#[async_trait]
impl AppSession for CliSession {
    /// Setup initial state for the session. Order is very important!!!
    async fn startup(&mut self) -> AppResult {
        self.console.set_reporter(DefaultReporter::default());

        // Determine paths

        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = if self.requires_workspace_setup() {
            startup::find_workspace_root(&self.working_dir)?
        } else {
            self.working_dir.clone()
        };

        // Load environments

        self.moon_env = startup::detect_moon_environment(&self.working_dir, &self.workspace_root)?;

        self.proto_env =
            startup::detect_proto_environment(&self.working_dir, &self.workspace_root)?;

        // Load configs

        if self.requires_workspace_setup() {
            let (workspace_config, tasks_config, toolchain_config) = try_join!(
                startup::load_workspace_config(self.config_loader.clone(), &self.workspace_root),
                startup::load_tasks_configs(self.config_loader.clone(), &self.workspace_root),
                startup::load_toolchain_config(
                    self.config_loader.clone(),
                    self.proto_env.clone(),
                    &self.workspace_root,
                    &self.working_dir,
                ),
            )?;

            self.workspace_config = workspace_config;
            self.toolchain_config = toolchain_config;
            self.tasks_config = tasks_config;
        }

        // Load components

        if !is_test_env() && is_ci() {
            let vcs = self.get_vcs_adapter()?;

            startup::signin_to_moonbase(&vcs).await?;
        }

        Ok(None)
    }

    /// Analyze the current state and install/registery necessary functionality.
    async fn analyze(&mut self) -> AppResult {
        if let Some(constraint) = &self.workspace_config.version_constraint {
            analyze::validate_version_constraint(constraint, &self.cli_version)?;
        }

        analyze::check_pkl_install()?;

        if self.requires_workspace_setup() {
            let cache_engine = self.get_cache_engine()?;

            analyze::install_proto(
                &self.console,
                &self.proto_env,
                &cache_engine,
                &self.toolchain_config,
            )
            .await?;

            analyze::register_platforms(
                &self.console,
                &self.proto_env,
                &self.toolchain_config,
                &self.workspace_root,
            )
            .await?;

            if self.requires_toolchain_installed() {
                analyze::load_toolchain(self.get_toolchain_registry().await?).await?;
            }
        }

        Ok(None)
    }

    async fn execute(&mut self) -> AppResult {
        if self.is_telemetry_enabled()
            && matches!(
                self.cli.command,
                Commands::Ci(_) | Commands::Check(_) | Commands::Run(_) | Commands::Sync { .. }
            )
        {
            let cache_engine = self.get_cache_engine()?;

            execute::check_for_new_version(
                &self.console,
                &self.moon_env,
                &cache_engine,
                &self.toolchain_config.moon.manifest_url,
            )
            .await?;
        }

        Ok(None)
    }

    async fn shutdown(&mut self) -> AppResult {
        self.console.close()?;

        Ok(None)
    }
}

impl fmt::Debug for CliSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliSession")
            .field("cli", &self.cli)
            .field("cli_version", &self.cli_version)
            .field("moon_env", &self.moon_env)
            .field("proto_env", &self.proto_env)
            .field("tasks_config", &self.tasks_config)
            .field("toolchain_config", &self.toolchain_config)
            .field("working_dir", &self.working_dir)
            .field("workspace_config", &self.workspace_config)
            .field("workspace_root", &self.workspace_root)
            .finish()
    }
}
