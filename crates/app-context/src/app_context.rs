use moon_cache::CacheEngine;
use moon_config::{ToolchainConfig, Version, WorkspaceConfig};
use moon_console::MoonConsole;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::BoxedVcs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppContext {
    pub cli_version: Version,

    // Components
    pub cache_engine: Arc<CacheEngine>,
    pub console: Arc<MoonConsole>,
    pub vcs: Arc<BoxedVcs>,

    // Configs
    pub toolchain_config: Arc<ToolchainConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Plugins
    pub toolchain_registry: Arc<ToolchainRegistry>,

    // Paths
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}
