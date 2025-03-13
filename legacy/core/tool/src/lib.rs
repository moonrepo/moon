mod errors;
mod manager;
mod tool;

pub use async_trait::async_trait;
pub use errors::*;
pub use manager::*;
use rustc_hash::FxHashMap;
use tokio::sync::{Mutex, RwLock};
pub use tool::*;

use moon_common::consts::PROTO_CLI_VERSION;
use proto_core::{
    Id, PluginLocator, ProtoEnvironment, Tool as ProtoTool, inject_proto_manifest_config,
};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use warpgate::{Wasm, inject_default_manifest_config};

pub fn use_global_tool_on_path(key: &str) -> bool {
    env::var("MOON_TOOLCHAIN_FORCE_GLOBALS").is_ok_and(|value| {
        if value == "1" || value == "true" || value == "on" || value == key {
            true
        } else if value.contains(",") {
            value.split(',').any(|val| val == key)
        } else {
            false
        }
    })
}

/// We need to ensure that our toolchain binaries are executed instead of
/// other binaries of the same name. Otherwise, tooling like nvm will
/// intercept execution and break our processes. We can work around this
/// by prepending the `PATH` environment variable.
pub fn prepend_path_env_var<I, V>(paths: I) -> std::ffi::OsString
where
    I: IntoIterator<Item = V>,
    V: AsRef<Path>,
{
    let path = env::var("PATH").unwrap_or_default();

    let mut paths = paths
        .into_iter()
        .map(|p| p.as_ref().to_path_buf())
        .collect::<Vec<_>>();

    paths.extend(env::split_paths(&path).collect::<Vec<_>>());

    env::join_paths(paths).unwrap()
}

pub fn get_proto_paths(proto: &ProtoEnvironment) -> Vec<PathBuf> {
    vec![
        // Always use a versioned proto first
        proto
            .store
            .inventory_dir
            .join("proto")
            .join(PROTO_CLI_VERSION),
        // Then fallback to shims/bins
        proto.store.shims_dir.clone(),
        proto.store.bin_dir.clone(),
        // And ensure non-proto managed moon comes last
        proto.home_dir.join(".moon").join("bin"),
    ]
}

pub fn get_proto_version_env(tool: &ProtoTool) -> Option<String> {
    tool.version.as_ref()?;

    let spec = tool.get_resolved_version();

    // If we have a "latest" alias, use "*" as a version instead,
    // otherwise latest will attempt to use a possibly uninstalled
    // version, while * will use any available/installed version.
    if spec.is_latest() {
        return Some("*".into());
    }

    Some(spec.to_string())
}

pub fn get_proto_env_vars() -> FxHashMap<String, String> {
    FxHashMap::from_iter([
        ("PROTO_AUTO_INSTALL".into(), "false".into()),
        ("PROTO_IGNORE_MIGRATE_WARNING".into(), "true".into()),
        ("PROTO_NO_PROGRESS".into(), "true".into()),
        // ("PROTO_LOG".into(), "trace".into()),
        ("PROTO_VERSION".into(), PROTO_CLI_VERSION.into()),
        ("STARBASE_FORCE_TTY".into(), "true".into()),
    ])
}

pub async fn load_tool_plugin(
    id: &Id,
    proto: &ProtoEnvironment,
    locator: &PluginLocator,
) -> miette::Result<ProtoTool> {
    let mut manifest = ProtoTool::create_plugin_manifest(
        proto,
        Wasm::file(proto.get_plugin_loader()?.load_plugin(id, locator).await?),
    )?;

    inject_default_manifest_config(id, &proto.home_dir, &mut manifest)?;
    inject_proto_manifest_config(id, proto, &mut manifest)?;

    ProtoTool::load_from_manifest(id, proto, manifest).await
}

static LOCKS: OnceLock<RwLock<FxHashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();

pub async fn get_shared_lock(id: &str) -> Arc<Mutex<()>> {
    let locks = LOCKS.get_or_init(|| RwLock::new(FxHashMap::default()));
    let mut map = locks.write().await;

    if let Some(lock) = map.get(id) {
        return lock.clone();
    }

    let lock = Arc::new(Mutex::new(()));

    map.insert(id.to_owned(), Arc::clone(&lock));

    lock
}
