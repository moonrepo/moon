use crate::app_error::AppError;
use crate::systems::*;
use async_trait::async_trait;
use moon_console::Console;
use moon_env::MoonEnvironment;
use proto_core::ProtoEnvironment;
use starbase::{AppResult, AppSession};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct MoonSession {
    pub console: Arc<Console>,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

#[async_trait]
impl AppSession for MoonSession {
    async fn startup(&mut self) -> AppResult {
        self.working_dir = env::current_dir().map_err(|_| AppError::MissingWorkingDir)?;

        self.workspace_root = startup::find_workspace_root(&self.working_dir)?;

        let mut env = MoonEnvironment::new()?;
        env.working_dir = self.working_dir.clone();
        env.workspace_root = self.workspace_root.clone();

        self.moon_env = Arc::new(env);

        let mut env = ProtoEnvironment::new()?;
        env.cwd = self.working_dir.clone();

        self.proto_env = Arc::new(env);

        Ok(())
    }
}
