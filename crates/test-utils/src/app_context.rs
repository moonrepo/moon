use moon_app_context::AppContext;
use moon_cache::CacheEngine;
use moon_config::{ConfigLoader, Version};
use moon_console::Console;
use moon_toolchain_plugin::ToolchainRegistry;
use moon_vcs::Git;
use proto_core::ProtoConfig;
use starbase_sandbox::create_sandbox;
use std::path::Path;
use std::sync::Arc;

pub fn generate_app_context(fixture: &str) -> AppContext {
    generate_app_context_from_sandbox(create_sandbox(fixture).path())
}

pub fn generate_app_context_from_sandbox(root: &Path) -> AppContext {
    let config_loader = ConfigLoader::default();
    let toolchain_config = config_loader
        .load_toolchain_config(root, &ProtoConfig::default())
        .unwrap();
    let toolchain_registry = ToolchainRegistry::default();
    let workspace_config = config_loader.load_workspace_config(root).unwrap();
    let vcs = Git::load(
        root,
        &workspace_config.vcs.default_branch,
        &workspace_config.vcs.remote_candidates,
    )
    .unwrap();

    AppContext {
        cli_version: Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
        cache_engine: Arc::new(CacheEngine::new(root).unwrap()),
        console: Arc::new(Console::new_testing()),
        toolchain_config: Arc::new(toolchain_config),
        toolchain_registry: Arc::new(toolchain_registry),
        vcs: Arc::new(Box::new(vcs)),
        working_dir: root.to_owned(),
        workspace_config: Arc::new(workspace_config),
        workspace_root: root.to_owned(),
    }
}
