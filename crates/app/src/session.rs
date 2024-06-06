use crate::app_error::AppError;
use crate::systems::*;
use async_trait::async_trait;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_env::MoonEnvironment;
use proto_core::ProtoEnvironment;
use starbase::{AppResult, AppSession};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct MoonSession {
    // Components
    pub console: Arc<Console>,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
    // vcs
    // engines
    // graphs
    // registries

    // Configs
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchain_config: ToolchainConfig,
    pub workspace_config: WorkspaceConfig,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}
impl MoonSession {
    pub fn new() -> Self {
        Self {
            console: Arc::new(Console::new(false)),
            moon_env: Arc::new(MoonEnvironment::default()),
            proto_env: Arc::new(ProtoEnvironment::new().unwrap()), // TODO
            tasks_config: Arc::new(InheritedTasksManager::default()),
            toolchain_config: ToolchainConfig::default(),
            working_dir: PathBuf::new(),
            workspace_root: PathBuf::new(),
            workspace_config: WorkspaceConfig::default(),
        }
    }
}

#[async_trait]
impl AppSession for MoonSession {
    /// Setup initial state for the session. Order is very important!!!
    async fn startup(&mut self) -> AppResult {
        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = startup::find_workspace_root(&self.working_dir)?;

        // Load environments

        self.moon_env = startup::detect_moon_environment(&self.working_dir, &self.workspace_root)?;

        self.proto_env =
            startup::detect_proto_environment(&self.working_dir, &self.workspace_root)?;

        // Load configs

        self.workspace_config = startup::load_workspace_config(&self.workspace_root)?;

        self.toolchain_config = startup::load_toolchain_config(
            &self.workspace_root,
            self.proto_env
                .load_config_manager()?
                .get_local_config(&self.working_dir)?,
        )?;

        self.tasks_config = startup::load_tasks_configs(&self.workspace_root)?;

        Ok(())
    }
}
