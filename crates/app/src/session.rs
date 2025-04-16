use crate::app::{Cli, Commands};
use crate::app_error::AppError;
use crate::commands::docker::DockerCommands;
use crate::components::*;
use crate::systems::*;
use async_trait::async_trait;
use moon_action_graph::{ActionGraphBuilder, ActionGraphBuilderOptions};
use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_common::is_formatted_output;
use moon_config::{ConfigLoader, InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::{Console, MoonReporter, create_console_theme};
use moon_env::MoonEnvironment;
use moon_extension_plugin::*;
use moon_feature_flags::{FeatureFlags, Flag};
use moon_plugin::PluginHostData;
use moon_process::ProcessRegistry;
use moon_project_graph::ProjectGraph;
use moon_task_graph::TaskGraph;
use moon_toolchain_plugin::*;
use moon_vcs::gitx::Gitx;
use moon_vcs::{BoxedVcs, Git};
use moon_workspace::WorkspaceBuilder;
use moon_workspace_graph::WorkspaceGraph;
use proto_core::ProtoEnvironment;
use semver::Version;
use starbase::{AppResult, AppSession};
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::try_join;
use tracing::debug;

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
    cache_engine: OnceLock<Arc<CacheEngine>>,
    extension_registry: OnceLock<Arc<ExtensionRegistry>>,
    project_graph: OnceLock<Arc<ProjectGraph>>,
    task_graph: OnceLock<Arc<TaskGraph>>,
    toolchain_registry: OnceLock<Arc<ToolchainRegistry>>,
    vcs_adapter: OnceLock<Arc<BoxedVcs>>,
    workspace_graph: OnceLock<Arc<WorkspaceGraph>>,

    // Configs
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl MoonSession {
    pub fn new(cli: Cli, cli_version: String) -> Self {
        debug!("Creating new application session");

        Self {
            cache_engine: OnceLock::new(),
            cli_version: Version::parse(&cli_version).unwrap(),
            config_loader: ConfigLoader::default(),
            console: Console::new(cli.quiet || is_formatted_output()),
            extension_registry: OnceLock::new(),
            moon_env: Arc::new(MoonEnvironment::default()),
            project_graph: OnceLock::new(),
            proto_env: Arc::new(ProtoEnvironment::default()),
            task_graph: OnceLock::new(),
            tasks_config: Arc::new(InheritedTasksManager::default()),
            toolchain_config: Arc::new(ToolchainConfig::default()),
            toolchain_registry: OnceLock::new(),
            working_dir: PathBuf::new(),
            workspace_config: Arc::new(WorkspaceConfig::default()),
            workspace_graph: OnceLock::new(),
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

    pub async fn get_app_context(&self) -> miette::Result<Arc<AppContext>> {
        Ok(Arc::new(AppContext {
            cli_version: self.cli_version.clone(),
            cache_engine: self.get_cache_engine()?,
            console: self.get_console()?,
            vcs: self.get_vcs_adapter()?,
            toolchain_config: Arc::clone(&self.toolchain_config),
            toolchain_registry: self.get_toolchain_registry().await?,
            workspace_config: Arc::clone(&self.workspace_config),
            working_dir: self.working_dir.clone(),
            workspace_root: self.workspace_root.clone(),
        }))
    }

    pub fn get_cache_engine(&self) -> miette::Result<Arc<CacheEngine>> {
        if self.cache_engine.get().is_none() {
            let _ = self
                .cache_engine
                .set(Arc::new(CacheEngine::new(&self.workspace_root)?));
        }

        Ok(self.cache_engine.get().map(Arc::clone).unwrap())
    }

    pub fn get_console(&self) -> miette::Result<Arc<Console>> {
        Ok(Arc::new(self.console.clone()))
    }

    pub async fn get_extension_registry(&self) -> miette::Result<Arc<ExtensionRegistry>> {
        let item = self.extension_registry.get_or_init(|| {
            let mut registry = ExtensionRegistry::new(PluginHostData {
                moon_env: Arc::clone(&self.moon_env),
                proto_env: Arc::clone(&self.proto_env),
                workspace_graph: Arc::new(OnceLock::new()),
            });

            registry.inherit_configs(&self.workspace_config.extensions);

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
        let item = self.toolchain_registry.get_or_init(|| {
            let mut registry = ToolchainRegistry::new(PluginHostData {
                moon_env: Arc::clone(&self.moon_env),
                proto_env: Arc::clone(&self.proto_env),
                workspace_graph: Arc::new(OnceLock::new()),
            });

            registry.inherit_configs(&self.toolchain_config.plugins);

            Arc::new(registry)
        });

        Ok(Arc::clone(item))
    }

    pub fn get_vcs_adapter(&self) -> miette::Result<Arc<BoxedVcs>> {
        if self.vcs_adapter.get().is_none() {
            let config = &self.workspace_config.vcs;

            let git: BoxedVcs = if FeatureFlags::instance().is_enabled(Flag::GitV2) {
                Box::new(Gitx::load(
                    &self.workspace_root,
                    &config.default_branch,
                    &config.remote_candidates,
                )?)
            } else {
                Box::new(Git::load(
                    &self.workspace_root,
                    &config.default_branch,
                    &config.remote_candidates,
                )?)
            };

            let _ = self.vcs_adapter.set(Arc::new(git));
        }

        Ok(self.vcs_adapter.get().map(Arc::clone).unwrap())
    }

    pub async fn get_workspace_graph(&self) -> miette::Result<Arc<WorkspaceGraph>> {
        if self.workspace_graph.get().is_none() {
            self.load_workspace_graph().await?;
        }

        Ok(self.workspace_graph.get().map(Arc::clone).unwrap())
    }

    pub fn is_telemetry_enabled(&self) -> bool {
        self.workspace_config.telemetry
    }

    pub fn requires_workspace_configured(&self) -> bool {
        !matches!(
            self.cli.command,
            Commands::Completions(_) | Commands::Init(_)
        )
    }

    pub fn requires_toolchain_installed(&self) -> bool {
        matches!(
            self.cli.command,
            Commands::Bin(_)
                | Commands::Docker {
                    command: DockerCommands::Prune
                }
                | Commands::Node { .. }
                | Commands::Teardown
        )
    }

    async fn load_workspace_graph(&self) -> miette::Result<()> {
        let cache_engine = self.get_cache_engine()?;
        let context = create_workspace_graph_context(self).await?;
        let builder = WorkspaceBuilder::new_with_cache(context, &cache_engine).await?;
        let workspace_graph = Arc::new(builder.build().await?);

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
        let _ = self.workspace_graph.set(workspace_graph);

        Ok(())
    }
}

#[async_trait]
impl AppSession for MoonSession {
    /// Setup initial state for the session. Order is very important!!!
    async fn startup(&mut self) -> AppResult {
        self.console.set_reporter(MoonReporter::default());
        self.console.set_theme(create_console_theme());

        startup::create_moonx_shims()?;

        // Determine paths

        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = if self.requires_workspace_configured() {
            startup::find_workspace_root(&self.working_dir)?
        } else {
            self.working_dir.clone()
        };

        // Load environments

        self.moon_env = startup::detect_moon_environment(&self.working_dir, &self.workspace_root)?;

        self.proto_env =
            startup::detect_proto_environment(&self.working_dir, &self.workspace_root)?;

        // Load configs

        if self.requires_workspace_configured() {
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

        startup::register_feature_flags(&self.workspace_config)?;

        ProcessRegistry::register(self.workspace_config.pipeline.kill_process_threshold);

        Ok(None)
    }

    /// Analyze the current state and install/registery necessary functionality.
    async fn analyze(&mut self) -> AppResult {
        if let Some(constraint) = &self.workspace_config.version_constraint {
            analyze::validate_version_constraint(constraint, &self.cli_version)?;
        }

        let vcs = self.get_vcs_adapter()?;

        analyze::extract_repo_info(&vcs).await?;

        if self.requires_workspace_configured() {
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
                analyze::load_toolchain().await?;
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
        // Ensure all child processes have finished running
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
            .field("toolchain_config", &self.toolchain_config)
            .field("working_dir", &self.working_dir)
            .field("workspace_config", &self.workspace_config)
            .field("workspace_root", &self.workspace_root)
            .finish()
    }
}
