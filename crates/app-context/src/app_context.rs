use moon_cache::CacheEngine;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{ExtensionsConfig, ToolchainsConfig, Version, WorkspaceConfig};
use moon_console::Console;
use moon_env::MoonEnvironment;
use moon_extension_plugin::ExtensionRegistry;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::BoxedVcs;
use proto_core::ProtoEnvironment;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppContext {
    pub cli_version: Version,
    pub moon_env: Arc<MoonEnvironment>,
    pub proto_env: Arc<ProtoEnvironment>,

    // Components
    pub cache_engine: Arc<CacheEngine>,
    pub console: Arc<Console>,
    pub vcs: Arc<BoxedVcs>,

    // Configs
    pub extensions_config: Arc<ExtensionsConfig>,
    pub toolchains_config: Arc<ToolchainsConfig>,
    pub workspace_config: Arc<WorkspaceConfig>,

    // Plugins
    pub extension_registry: Arc<ExtensionRegistry>,
    pub toolchain_registry: Arc<ToolchainRegistry>,

    // Paths
    pub config_dir: PathBuf,
    pub daemon_dir: PathBuf,
    pub working_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl AppContext {
    pub async fn hash_files(
        &self,
        files: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        if true {
            self.cache_engine
                .hash_files(&self.workspace_root, files)
                .await
        } else {
            self.vcs.get_file_hashes(files, true).await
        }
    }
}
