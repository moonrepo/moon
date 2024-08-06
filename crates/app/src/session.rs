use crate::app::{Cli, Commands};
use crate::app_error::AppError;
use crate::components::*;
use crate::systems::*;
use async_trait::async_trait;
use moon_action_graph::ActionGraphBuilder;
use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_common::{is_ci, is_test_env};
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_console_reporter::DefaultReporter;
use moon_env::MoonEnvironment;
use moon_extension_plugin::*;
use moon_project_graph::{ProjectGraph, ProjectGraphBuilder};
use moon_toolchain_plugin::*;
use moon_vcs::{BoxedVcs, Git};
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
    pub console: Console,
    pub moonbase: Option<Arc<Moonbase>>,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,

    // Lazy components
    cache_engine: OnceCell<Arc<CacheEngine>>,
    extension_registry: OnceCell<Arc<ExtensionRegistry>>,
    project_graph: OnceCell<Arc<ProjectGraph>>,
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
            console: Console::new(cli.quiet),
            extension_registry: OnceCell::new(),
            moonbase: None,
            moon_env: Arc::new(MoonEnvironment::default()),
            project_graph: OnceCell::new(),
            proto_env: Arc::new(ProtoEnvironment::default()),
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
        project_graph: &'graph ProjectGraph,
    ) -> AppResult<ActionGraphBuilder<'graph>> {
        ActionGraphBuilder::new(project_graph)
    }

    pub async fn build_project_graph(&self) -> AppResult<ProjectGraphBuilder> {
        ProjectGraphBuilder::new(create_project_graph_context(self).await?).await
    }

    pub fn get_app_context(&self) -> AppResult<Arc<AppContext>> {
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

    pub fn get_cache_engine(&self) -> AppResult<Arc<CacheEngine>> {
        let item = self
            .cache_engine
            .get_or_try_init(|| CacheEngine::new(&self.workspace_root).map(Arc::new))?;

        Ok(Arc::clone(item))
    }

    pub fn get_console(&self) -> AppResult<Arc<Console>> {
        Ok(Arc::new(self.console.clone()))
    }

    pub fn get_extension_registry(&self) -> AppResult<Arc<ExtensionRegistry>> {
        let item = self.extension_registry.get_or_init(|| {
            Arc::new(ExtensionRegistry::new(
                Arc::clone(&self.moon_env),
                Arc::clone(&self.proto_env),
            ))
        });

        Ok(Arc::clone(item))
    }

    pub async fn get_project_graph(&self) -> AppResult<Arc<ProjectGraph>> {
        if let Some(item) = self.project_graph.get() {
            return Ok(Arc::clone(item));
        }

        let cache_engine = self.get_cache_engine()?;
        let context = create_project_graph_context(self).await?;
        let builder = ProjectGraphBuilder::generate(context, &cache_engine).await?;
        let graph = Arc::new(builder.build().await?);

        let _ = self.project_graph.set(Arc::clone(&graph));

        Ok(graph)
    }

    pub fn get_toolchain_registry(&self) -> AppResult<Arc<ToolchainRegistry>> {
        let item = self.toolchain_registry.get_or_init(|| {
            Arc::new(ToolchainRegistry::new(
                Arc::clone(&self.moon_env),
                Arc::clone(&self.proto_env),
            ))
        });

        Ok(Arc::clone(item))
    }

    pub fn get_vcs_adapter(&self) -> AppResult<Arc<BoxedVcs>> {
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

    pub fn is_telemetry_enabled(&self) -> bool {
        self.workspace_config.telemetry
    }

    pub fn requires_workspace_setup(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Completions(_) | Commands::Init(_) | Commands::Setup | Commands::Upgrade
        )
    }

    pub fn requires_toolchain_installed(&self) -> bool {
        matches!(
            self.cli.command,
            Commands::Bin(_) | Commands::Docker { .. } | Commands::Node { .. } | Commands::Teardown
        )
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
                startup::load_workspace_config(&self.workspace_root),
                startup::load_tasks_configs(&self.workspace_root),
                startup::load_toolchain_config(
                    &self.workspace_root,
                    &self.working_dir,
                    self.proto_env.clone(),
                ),
            )?;

            self.workspace_config = workspace_config;
            self.toolchain_config = toolchain_config;
            self.tasks_config = tasks_config;
        }

        // Load components

        if !is_test_env() && is_ci() {
            let vcs = self.get_vcs_adapter()?;

            self.moonbase = startup::signin_to_moonbase(&vcs).await?;
        }

        Ok(())
    }

    /// Analyze the current state and install/registery necessary functionality.
    async fn analyze(&mut self) -> AppResult {
        if let Some(constraint) = &self.workspace_config.version_constraint {
            analyze::validate_version_constraint(constraint, &self.cli_version)?;
        }

        if self.requires_workspace_setup() {
            self.get_cache_engine()?;

            analyze::install_proto(&self.console, &self.proto_env, &self.toolchain_config).await?;

            analyze::register_platforms(
                &self.console,
                &self.proto_env,
                &self.toolchain_config,
                &self.workspace_root,
            )
            .await?;

            if self.requires_toolchain_installed() {
                analyze::load_toolchain(self.get_toolchain_registry()?).await?;
            }
        }

        Ok(())
    }

    async fn execute(&mut self) -> AppResult {
        if self.is_telemetry_enabled()
            && matches!(
                self.cli.command,
                Commands::Ci(_) | Commands::Check(_) | Commands::Run(_) | Commands::Sync { .. }
            )
        {
            let cache_engine = self.get_cache_engine()?;

            execute::check_for_new_version(&self.console, &self.moon_env, &cache_engine).await?;
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> AppResult {
        self.console.close()?;

        Ok(())
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
