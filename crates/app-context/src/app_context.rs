use moon_cache::CacheEngine;
use moon_config::{ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_vcs::BoxedVcs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext {
    // Components
    pub cache_engine: Arc<CacheEngine>,
    pub console: Arc<Console>,
    pub vcs: Arc<BoxedVcs>,

    // Configs
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}
