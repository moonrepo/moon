use moon_config::{ExtensionsConfig, ToolchainsConfig, WorkspaceConfig};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct GraphExpanderContext {
    pub config_dir: PathBuf,
    pub extensions_config: Arc<ExtensionsConfig>,
    pub toolchains_config: Arc<ToolchainsConfig>,
    pub vcs_branch: Arc<String>,
    pub vcs_repository: Arc<String>,
    pub vcs_revision: Arc<String>,
    pub working_dir: PathBuf,
    pub workspace_config: Arc<WorkspaceConfig>,
    pub workspace_root: PathBuf,
}
