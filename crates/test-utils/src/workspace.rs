use moon_cache::CacheEngine;
use moon_config::{InheritedTasksManager, ToolchainConfig, WorkspaceConfig};
use moon_vcs::Git;
use moon_workspace::Workspace;
use proto_core::ProtoConfig;
use starbase_sandbox::create_sandbox;
use std::path::Path;
use std::sync::Arc;

pub fn generate_workspace(fixture: &str) -> Workspace {
    generate_workspace_from_sandbox(create_sandbox(fixture).path())
}

pub fn generate_workspace_from_sandbox(root: &Path) -> Workspace {
    let tasks_config = InheritedTasksManager::load_from(root).unwrap();
    let toolchain_config = ToolchainConfig::load_from(root, &ProtoConfig::default()).unwrap();
    let config = WorkspaceConfig::load_from(root).unwrap();
    let vcs = Git::load(
        root,
        &config.vcs.default_branch,
        &config.vcs.remote_candidates,
    )
    .unwrap();

    Workspace {
        cache_engine: Arc::new(CacheEngine::new(root).unwrap()),
        config: Arc::new(config),
        root: root.to_owned(),
        session: None,
        tasks_config: Arc::new(tasks_config),
        toolchain_config: Arc::new(toolchain_config),
        vcs: Arc::new(Box::new(vcs)),
        working_dir: root.to_owned(),
    }
}
