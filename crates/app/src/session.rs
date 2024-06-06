use crate::app_error::AppError;
use crate::systems::*;
use async_trait::async_trait;
use moon_cache::CacheEngine;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_env::MoonEnvironment;
use moon_vcs::{BoxedVcs, Git};
use once_cell::sync::OnceCell;
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
    // graphs
    // registries

    // Lazy components
    cache_engine: OnceCell<Arc<CacheEngine>>,
    vcs_adapter: OnceCell<Arc<BoxedVcs>>,

    // Configs
    pub tasks_config: Arc<InheritedTasksManager>,
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}
impl MoonSession {
    pub fn new() -> Self {
        Self {
            cache_engine: OnceCell::new(),
            console: Arc::new(Console::new(false)),
            moon_env: Arc::new(MoonEnvironment::default()),
            proto_env: Arc::new(ProtoEnvironment::new().unwrap()), // TODO
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
