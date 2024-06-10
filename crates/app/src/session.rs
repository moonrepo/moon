use crate::app_error::AppError;
use crate::systems::*;
use async_trait::async_trait;
use moon_cache::CacheEngine;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_console_reporter::DefaultReporter;
use moon_env::MoonEnvironment;
use moon_extension_plugin::ExtensionPlugin;
use moon_plugin::PluginRegistry;
use moon_plugin::PluginType;
use moon_vcs::{BoxedVcs, Git};
use once_cell::sync::OnceCell;
use proto_core::ProtoEnvironment;
use starbase::{AppResult, AppSession};
use std::env;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::try_join;

pub type ExtensionRegistry = PluginRegistry<ExtensionPlugin>;

#[derive(Clone)]
pub struct CliSession {
    // Components
    pub console: Console,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,

    // Lazy components
    cache_engine: OnceCell<Arc<CacheEngine>>,
    extension_registry: OnceCell<Arc<ExtensionRegistry>>,
    vcs_adapter: OnceCell<Arc<BoxedVcs>>,
    // graphs
    // registries

    // Configs
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl CliSession {
    pub fn new() -> Self {
        Self {
            cache_engine: OnceCell::new(),
            console: Console::new(false),
            extension_registry: OnceCell::new(),
            moon_env: Arc::new(MoonEnvironment::default()),
            proto_env: Arc::new(ProtoEnvironment::default()),
            tasks_config: Arc::new(InheritedTasksManager::default()),
            toolchain_config: Arc::new(ToolchainConfig::default()),
            working_dir: PathBuf::new(),
            workspace_root: PathBuf::new(),
            workspace_config: Arc::new(WorkspaceConfig::default()),
            vcs_adapter: OnceCell::new(),
        }
    }

    pub fn get_cache_engine(&self) -> AppResult<Arc<CacheEngine>> {
        let item = self
            .cache_engine
            .get_or_try_init(|| CacheEngine::new(&self.workspace_root).map(Arc::new))?;

        Ok(Arc::clone(item))
    }

    pub fn get_extension_registry(&self) -> AppResult<Arc<ExtensionRegistry>> {
        let item = self.extension_registry.get_or_init(|| {
            Arc::new(PluginRegistry::new(
                PluginType::Extension,
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

    pub fn requires_workspace(&self) -> bool {
        true // TODO
    }

    pub fn requires_toolchain(&self) -> bool {
        false // TODO
    }
}

#[async_trait]
impl AppSession for CliSession {
    /// Setup initial state for the session. Order is very important!!!
    async fn startup(&mut self) -> AppResult {
        self.console.set_reporter(DefaultReporter::default());

        // Determine paths

        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = startup::find_workspace_root(&self.working_dir)?;

        // Load environments

        self.moon_env = startup::detect_moon_environment(&self.working_dir, &self.workspace_root)?;

        self.proto_env =
            startup::detect_proto_environment(&self.working_dir, &self.workspace_root)?;

        // Load configs

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

        // TODO moonbase

        Ok(())
    }

    /// Analyze the current state and install/registery necessary functionality.
    async fn analyze(&mut self) -> AppResult {
        analyze::prepate_repository(self.get_vcs_adapter()?).await?;

        if self.requires_workspace() {
            analyze::install_proto(&self.console, &self.proto_env, &self.toolchain_config).await?;

            analyze::register_platforms(
                &self.console,
                &self.proto_env,
                &self.toolchain_config,
                &self.workspace_root,
            )
            .await?;

            if self.requires_toolchain() {
                analyze::load_toolchain().await?;
            }
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
        f.debug_struct("MoonSession")
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
