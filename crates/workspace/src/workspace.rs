use moon_api::Moonbase;
use moon_cache::CacheEngine;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_vcs::BoxedVcs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct Workspace {
    /// Engine for reading and writing cache/states.
    pub cache_engine: Arc<CacheEngine>,

    /// Workspace configuration loaded from ".moon/workspace.yml".
    pub config: Arc<WorkspaceConfig>,

    // /// Local `.prototools` config.
    // pub proto_config: Arc<ProtoConfig>,
    /// The root of the workspace that contains the ".moon" config folder.
    pub root: PathBuf,

    /// When logged in, the auth token and IDs for making API requests.
    pub session: Option<Arc<Moonbase>>,

    /// Global tasks configuration loaded from ".moon/tasks.yml".
    pub tasks_config: Arc<InheritedTasksManager>,

    /// Toolchain configuration loaded from ".moon/toolchain.yml".
    pub toolchain_config: Arc<ToolchainConfig>,

    /// Configured version control system.
    pub vcs: Arc<BoxedVcs>,

    /// The current working directory.
    pub working_dir: PathBuf,
}
