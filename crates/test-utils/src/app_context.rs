use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_config::{ToolchainConfig, WorkspaceConfig};
use moon_console::Console;
use moon_vcs::Git;
use proto_core::ProtoConfig;
use starbase_sandbox::create_sandbox;
use std::path::Path;
use std::sync::Arc;

pub fn generate_app_context(fixture: &str) -> AppContext {
    generate_app_context_from_sandbox(create_sandbox(fixture).path())
}

pub fn generate_app_context_from_sandbox(root: &Path) -> AppContext {
    let toolchain_config = ToolchainConfig::load_from(root, &ProtoConfig::default()).unwrap();
    let workspace_config = WorkspaceConfig::load_from(root).unwrap();
    let vcs = Git::load(
        root,
        &workspace_config.vcs.default_branch,
        &workspace_config.vcs.remote_candidates,
    )
    .unwrap();

    AppContext {
        cache_engine: Arc::new(CacheEngine::new(root).unwrap()),
        console: Arc::new(Console::new_testing()),
        toolchain_config: Arc::new(toolchain_config),
        vcs: Arc::new(Box::new(vcs)),
        working_dir: root.to_owned(),
        workspace_config: Arc::new(workspace_config),
        workspace_root: root.to_owned(),
    }
}
