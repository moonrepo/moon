use crate::app::{Cli, Commands};
use crate::app_error::AppError;
use crate::systems::*;
use async_trait::async_trait;
use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_api::Launchpad;
use moon_app_context::AppContext;
use moon_cache::{CacheContext, CacheEngine};
use moon_cache_local::LocalStorage;
use moon_cache_remote::{GrpcRemoteStorage, HttpRemoteStorage};
use moon_codegen::CodeGenerator;
use moon_common::{is_ci_env, is_docker, is_formatted_output, is_test_env};
use moon_config::{
    ExtensionsConfig, InheritedTasksManager, RemoteApi, ToolchainsConfig, WorkspaceConfig,
};
use moon_config_loader::ConfigLoader;
use moon_console::{Console, MoonReporter, create_console_theme};
use moon_daemon::{DaemonClient, DaemonConnector};
use moon_env::MoonEnvironment;
use moon_env_var::GlobalEnvBag;
use moon_extension_plugin::*;
use moon_plugin::MoonHostData;
use moon_process::ProcessRegistry;
use moon_project_graph::ProjectGraph;
use moon_task_graph::TaskGraph;
use moon_toolchain_plugin::*;
use moon_vcs::{BoxedVcs, git::Git};
use moon_workspace::{WorkspaceBuilder, WorkspaceBuilderAsync, WorkspaceBuilderContext};
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoEnvironment;
use semver::Version;
use starbase::{AppResult, AppSession};
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::OnceCell;
use tokio::try_join;
use tracing::debug;

pub type SessionResult = AppResult<miette::Report>;

#[derive(Clone)]
pub struct MoonSession {
    pub cli: Cli,
    pub cli_version: Version,

    // Components
    pub config_loader: ConfigLoader,
    pub console: Console,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,

    // Lazy components
    pub(crate) cache_engine: OnceLock<Arc<CacheEngine>>,
    pub(crate) daemon_client: OnceLock<DaemonClient>,
    pub(crate) extension_registry: OnceCell<Arc<ExtensionRegistry>>,
    pub(crate) project_graph: OnceLock<Arc<ProjectGraph>>,
    pub(crate) task_graph: OnceLock<Arc<TaskGraph>>,
    pub(crate) toolchain_registry: OnceCell<Arc<ToolchainRegistry>>,
    pub(crate) vcs_adapter: OnceLock<Arc<BoxedVcs>>,
    pub(crate) workspace_graph: OnceCell<Arc<WorkspaceGraph>>,

    // Configs
    pub extensions_config: Arc<ExtensionsConfig>,
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchains_config: Arc<ToolchainsConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub config_dir: PathBuf,
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl MoonSession {
    pub fn new(cli: Cli, cli_version: String) -> Self {
        debug!("Creating new application session");

        Self {
            cache_engine: OnceLock::new(),
            cli_version: Version::parse(&cli_version).unwrap(),
            config_dir: PathBuf::new(),
            config_loader: ConfigLoader::default(),
            console: Console::new(cli.quiet || is_formatted_output()),
            daemon_client: OnceLock::new(),
            extensions_config: Arc::new(ExtensionsConfig::default()),
            extension_registry: OnceCell::new(),
            moon_env: Arc::new(MoonEnvironment::default()),
            project_graph: OnceLock::new(),
            proto_env: Arc::new(ProtoEnvironment::default()),
            task_graph: OnceLock::new(),
            tasks_config: Arc::new(InheritedTasksManager::default()),
            toolchains_config: Arc::new(ToolchainsConfig::default()),
            toolchain_registry: OnceCell::new(),
            working_dir: PathBuf::new(),
            workspace_config: Arc::new(WorkspaceConfig::default()),
            workspace_graph: OnceCell::new(),
            workspace_root: PathBuf::new(),
            vcs_adapter: OnceLock::new(),
            cli,
        }
    }

    pub async fn build_action_graph<'graph>(&self) -> miette::Result<ActionGraphBuilder<'graph>> {
        let config = &self.workspace_config.pipeline;

        self.build_action_graph_with_options(ActionGraphBuilderOptions {
            install_dependencies: config.install_dependencies.clone(),
            setup_environment: true.into(),
            setup_toolchains: true.into(),
            sync_projects: config.sync_projects.clone(),
            sync_project_dependencies: config.sync_project_dependencies,
            sync_workspace: config.sync_workspace,
        })
        .await
    }

    pub async fn build_action_graph_with_options<'graph>(
        &self,
        options: ActionGraphBuilderOptions,
    ) -> miette::Result<ActionGraphBuilder<'graph>> {
        let app_context = self.get_app_context().await?;
        let workspace_graph = self.get_workspace_graph().await?;

        ActionGraphBuilder::new(app_context, workspace_graph, options)
    }

    pub fn build_code_generator(&self) -> CodeGenerator<'_> {
        CodeGenerator::new(
            &self.workspace_root,
            &self.workspace_config.generator,
            Arc::clone(&self.moon_env),
        )
    }

    pub async fn connect_to_daemon(&self) -> miette::Result<Option<DaemonClient>> {
        if !self.is_daemon_allowed() {
            return Ok(None);
        }

        if let Some(client) = self.daemon_client.get() {
            return Ok(Some(client.to_owned()));
        }

        let client = self.get_daemon_connector()?.acquire().await?;

        // Only cache the client if we successfully connected to a daemon.
        // If we failed to connect, we don't want to cache so that we try again.
        if let Some(client) = &client {
            let _ = self.daemon_client.set(client.to_owned());
        }

        Ok(client)
    }

    pub async fn create_workspace_graph_context(&self) -> miette::Result<WorkspaceBuilderContext> {
        Ok(WorkspaceBuilderContext {
            cache_engine: self.get_cache_engine()?,
            config_loader: self.config_loader.clone(),
            enabled_toolchains: self.toolchains_config.get_enabled(),
            extensions_config: Arc::clone(&self.extensions_config),
            extension_registry: self.get_extension_registry().await?,
            inherited_tasks: Arc::clone(&self.tasks_config),
            toolchains_config: Arc::clone(&self.toolchains_config),
            toolchain_registry: self.get_toolchain_registry().await?,
            vcs: Some(self.get_vcs_adapter()?),
            working_dir: self.working_dir.clone(),
            workspace_config: Arc::clone(&self.workspace_config),
            workspace_root: self.workspace_root.clone(),
        })
    }

    pub async fn get_app_context(&self) -> miette::Result<Arc<AppContext>> {
        Ok(Arc::new(AppContext {
            cli_version: self.cli_version.clone(),
            cache_engine: self.get_cache_engine()?,
            config_dir: self.config_dir.clone(),
            config_exts: self.config_loader.extensions.clone(),
            console: self.get_console()?,
            daemon_dir: self.config_dir.join("cache").join("daemon"),
            moon_env: Arc::clone(&self.moon_env),
            proto_env: Arc::clone(&self.proto_env),
            extensions_config: Arc::clone(&self.extensions_config),
            extension_registry: self.get_extension_registry().await?,
            toolchains_config: Arc::clone(&self.toolchains_config),
            toolchain_registry: self.get_toolchain_registry().await?,
            vcs: self.get_vcs_adapter()?,
            working_dir: self.working_dir.clone(),
            workspace_config: Arc::clone(&self.workspace_config),
            workspace_root: self.workspace_root.clone(),
        }))
    }

    pub fn get_cache_engine(&self) -> miette::Result<Arc<CacheEngine>> {
        if self.cache_engine.get().is_none() {
            let context = CacheContext {
                cache_dir: self.config_dir.join("cache"),
                cache_config: Arc::new(self.workspace_config.cache.clone()),
                config_dir: self.config_dir.clone(),
                remote_config: Arc::new(self.workspace_config.remote.clone()),
                remote_debug: GlobalEnvBag::instance().should_debug_remote(),
                workspace_root: self.workspace_root.clone(),
            };

            let mut engine = CacheEngine::new(context.clone())?;

            if self.workspace_config.experiments.cas_outputs_cache {
                engine.storage.add_local_backend(LocalStorage::new(
                    context.clone(),
                    &context.cache_dir,
                    false,
                )?);
            }

            if context.remote_config.is_enabled() {
                match context.remote_config.api {
                    RemoteApi::Grpc => {
                        engine
                            .storage
                            .add_remote_backend(GrpcRemoteStorage::new(context.clone())?);
                    }
                    RemoteApi::Http => {
                        engine
                            .storage
                            .add_remote_backend(HttpRemoteStorage::new(context.clone())?);
                    }
                };
            }

            let _ = self.cache_engine.set(Arc::new(engine));
        }

        Ok(self.cache_engine.get().map(Arc::clone).unwrap())
    }

    pub fn get_console(&self) -> miette::Result<Arc<Console>> {
        Ok(Arc::new(self.console.clone()))
    }

    pub fn get_daemon_connector(&self) -> miette::Result<DaemonConnector> {
        Ok(DaemonConnector::new(
            self.config_dir.join("cache").join("daemon"),
            self.workspace_root.clone(),
            self.cli_version.to_string(),
        ))
    }

    pub async fn get_extension_registry(&self) -> miette::Result<Arc<ExtensionRegistry>> {
        self.extension_registry
            .get_or_try_init(async || {
                Ok(Arc::new(ExtensionRegistry::new(
                    MoonHostData {
                        moon_env: Arc::clone(&self.moon_env),
                        proto_env: Arc::clone(&self.proto_env),
                        extensions_config: Arc::clone(&self.extensions_config),
                        toolchains_config: Arc::clone(&self.toolchains_config),
                        workspace_config: Arc::clone(&self.workspace_config),
                        workspace_graph: Arc::new(OnceLock::new()),
                    },
                    Arc::clone(&self.extensions_config),
                )?))
            })
            .await
            .map(Arc::clone)
    }

    pub async fn get_project_graph(&self) -> miette::Result<Arc<ProjectGraph>> {
        if self.project_graph.get().is_none() {
            self.get_workspace_graph().await?;
        }

        Ok(self.project_graph.get().map(Arc::clone).unwrap())
    }

    pub async fn get_task_graph(&self) -> miette::Result<Arc<TaskGraph>> {
        if self.task_graph.get().is_none() {
            self.get_workspace_graph().await?;
        }

        Ok(self.task_graph.get().map(Arc::clone).unwrap())
    }

    pub async fn get_toolchain_registry(&self) -> miette::Result<Arc<ToolchainRegistry>> {
        self.toolchain_registry
            .get_or_try_init(async || {
                Ok(Arc::new(ToolchainRegistry::new(
                    MoonHostData {
                        moon_env: Arc::clone(&self.moon_env),
                        proto_env: Arc::clone(&self.proto_env),
                        extensions_config: Arc::clone(&self.extensions_config),
                        toolchains_config: Arc::clone(&self.toolchains_config),
                        workspace_config: Arc::clone(&self.workspace_config),
                        workspace_graph: Arc::new(OnceLock::new()),
                    },
                    Arc::clone(&self.toolchains_config),
                )?))
            })
            .await
            .map(Arc::clone)
    }

    pub fn get_vcs_adapter(&self) -> miette::Result<Arc<BoxedVcs>> {
        if self.vcs_adapter.get().is_none() {
            let config = &self.workspace_config.vcs;

            let git: BoxedVcs = Box::new(Git::load(
                &self.workspace_root,
                &config.default_branch,
                &config.remote_candidates,
            )?);

            let _ = self.vcs_adapter.set(Arc::new(git));
        }

        Ok(self.vcs_adapter.get().map(Arc::clone).unwrap())
    }

    pub async fn get_workspace_graph(&self) -> miette::Result<Arc<WorkspaceGraph>> {
        self.workspace_graph
            .get_or_try_init(async || self.load_workspace_graph().await)
            .await
            .map(Arc::clone)
    }

    pub fn is_daemon_allowed(&self) -> bool {
        self.workspace_config.daemon && self.is_pipeline_command() && !is_docker() && !is_test_env()
    }

    pub fn is_pipeline_command(&self) -> bool {
        matches!(
            self.cli.command,
            Commands::Ci(_)
                | Commands::Check(_)
                | Commands::Exec(_)
                | Commands::Run(_)
                | Commands::Sync { .. }
        )
    }

    pub fn is_telemetry_enabled(&self) -> bool {
        self.workspace_config.telemetry
    }

    pub fn requires_workspace_configured(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Completions(_) | Commands::Init(_) | Commands::Migrate { .. }
        )
    }

    async fn load_workspace_graph(&self) -> miette::Result<Arc<WorkspaceGraph>> {
        let cache_engine = self.get_cache_engine()?;
        let context = self.create_workspace_graph_context().await?;

        let workspace_graph = Arc::new(if self.workspace_config.experiments.async_graph_building {
            WorkspaceBuilderAsync::new_with_cache(context, &cache_engine)
                .await?
                .build()
                .await?
        } else {
            WorkspaceBuilder::new_with_cache(context, &cache_engine)
                .await?
                .build()
                .await?
        });

        // Update the plugin registries with the graph
        let extensions = self.get_extension_registry().await?;
        let _ = extensions
            .host_data
            .workspace_graph
            .set(workspace_graph.clone());

        let toolchains = self.get_toolchain_registry().await?;
        let _ = toolchains
            .host_data
            .workspace_graph
            .set(workspace_graph.clone());

        // Set the internal graphs
        let _ = self.project_graph.set(workspace_graph.projects.clone());
        let _ = self.task_graph.set(workspace_graph.tasks.clone());
        let _ = self.workspace_graph.set(workspace_graph.clone());

        Ok(workspace_graph)
    }
}

#[async_trait]
impl AppSession for MoonSession {
    type Error = miette::Report;

    /// Setup initial state for the session. Order is very important!!!
    async fn startup(&mut self) -> AppResult<Self::Error> {
        self.console.set_reporter(MoonReporter::default());
        self.console.set_theme(create_console_theme());

        // Determine paths

        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = if self.requires_workspace_configured() {
            startup::find_workspace_root(&self.working_dir)?
        } else {
            self.working_dir.clone()
        };

        self.config_dir = self.config_loader.locate_dir(&self.workspace_root);

        // Load environments

        self.moon_env = startup::detect_moon_environment(&self.working_dir, &self.workspace_root)?;

        self.proto_env =
            startup::detect_proto_environment(&self.working_dir, &self.workspace_root)?;

        // Load configs

        if self.requires_workspace_configured() {
            let (workspace_config, tasks_config, extensions_config, toolchains_config) = try_join!(
                startup::load_workspace_config(self.config_loader.clone(), &self.workspace_root),
                startup::load_tasks_configs(self.config_loader.clone(), &self.workspace_root),
                startup::load_extensions_config(self.config_loader.clone(), &self.workspace_root),
                startup::load_toolchains_config(
                    self.config_loader.clone(),
                    self.proto_env.clone(),
                    &self.workspace_root,
                    &self.working_dir,
                ),
            )?;

            self.workspace_config = workspace_config;
            self.extensions_config = extensions_config;
            self.toolchains_config = toolchains_config;
            self.tasks_config = tasks_config;
        }

        startup::register_feature_flags(&self.workspace_config)?;

        // Load singleton components
        ProcessRegistry::register(self.workspace_config.pipeline.kill_process_threshold);

        if self.requires_workspace_configured() {
            Launchpad::register(self.moon_env.clone())?;
        }

        Ok(None)
    }

    /// Analyze the current state and install/registery necessary functionality.
    async fn analyze(&mut self) -> AppResult<Self::Error> {
        if let Some(constraint) = &self.workspace_config.version_constraint {
            analyze::validate_version_constraint(constraint, &self.cli_version)?;
        }

        let vcs = self.get_vcs_adapter()?;

        analyze::extract_repo_info(&vcs).await?;

        // Preload components
        if self.requires_workspace_configured() {
            let _ = self.get_cache_engine()?;
        }

        Ok(None)
    }

    // This function runs in an async task (background thread)
    async fn execute(&mut self) -> AppResult<Self::Error> {
        // Check for a new version and log to the console
        if self.is_telemetry_enabled() && self.is_pipeline_command() {
            execute::check_for_new_version(&self, &self.toolchains_config.moon.manifest_url)
                .await?;
        }

        // Start the daemon early, in the background, so it's ready by the time
        // the pipeline needs it. This shares `acquire` with the pipeline's own
        // `connect_to_daemon`, so the two coordinate and at most one spawns —
        // and a spawn failure degrades to `None` instead of failing the run.
        if self.is_daemon_allowed()
            && let Some(client) = self.get_daemon_connector()?.acquire().await?
        {
            let _ = self.daemon_client.set(client);
        }

        Ok(None)
    }

    async fn shutdown(&mut self) -> AppResult<Self::Error> {
        let should_stop_daemon = cfg!(debug_assertions) || is_ci_env();

        // Stop the daemon if it's running. Connect fresh rather than reuse the
        // cached client — the cache is set on the execute-phase session clone,
        // not the one running shutdown — using a single attempt so we don't
        // spawn a daemon just to stop it.
        if should_stop_daemon
            && self.is_daemon_allowed()
            && let Ok(connector) = self.get_daemon_connector()
            && let Ok(Some(mut daemon)) = connector.connect_once().await
        {
            let _ = daemon.stop().await;
        }

        // Ensure all in-flight storage tasks have finished
        self.get_cache_engine()?
            .storage
            .wait_for_background_tasks()
            .await?;

        // Ensure all child processes have finished
        ProcessRegistry::instance()
            .wait_for_running_to_shutdown()
            .await;

        self.console.close()?;

        Ok(None)
    }
}

impl fmt::Debug for MoonSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CliSession")
            .field("cli", &self.cli)
            .field("cli_version", &self.cli_version)
            .field("moon_env", &self.moon_env)
            .field("proto_env", &self.proto_env)
            .field("tasks_config", &self.tasks_config)
            .field("extensions_config", &self.extensions_config)
            .field("toolchains_config", &self.toolchains_config)
            .field("working_dir", &self.working_dir)
            .field("workspace_config", &self.workspace_config)
            .field("workspace_root", &self.workspace_root)
            .finish()
    }
}
