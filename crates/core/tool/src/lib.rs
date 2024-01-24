mod errors;
mod manager;
mod tool;

pub use async_trait::async_trait;
pub use errors::*;
pub use manager::*;
use rustc_hash::FxHashMap;
pub use tool::*;

use moon_common::consts::PROTO_CLI_VERSION;
use proto_core::{
    inject_proto_manifest_config, Id, PluginLocator, ProtoEnvironment, Tool as ProtoTool,
};
use std::env;
use std::path::{Path, PathBuf};
use warpgate::{inject_default_manifest_config, Wasm};

pub fn use_global_tool_on_path() -> bool {
    env::var("MOON_TOOLCHAIN_FORCE_GLOBALS").is_ok_and(|v| v == "1" || v == "true" || v == "on")
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
        // For debugging
        // proto.home.join("Projects").join("proto").join("target").join("debug"),
        // Always use a versioned proto first
        proto.tools_dir.join("proto").join(PROTO_CLI_VERSION),
        // Then fallback to shims/bins
        proto.shims_dir.clone(),
        proto.bin_dir.clone(),
        // And ensure non-proto managed moon comes last
        proto.home.join(".moon").join("bin"),
    ]
}

pub fn get_proto_version_env(tool: &ProtoTool) -> Option<String> {
    let spec = tool.get_resolved_version();

    if spec.is_latest() {
        return None;
    }

    Some(spec.to_string())
}

pub fn get_proto_env_vars() -> FxHashMap<String, String> {
    FxHashMap::from_iter([
        ("PROTO_IGNORE_MIGRATE_WARNING".into(), "true".into()),
        ("PROTO_NO_PROGRESS".into(), "true".into()),
        // ("PROTO_LOG".into(), "trace".into()),
        ("PROTO_VERSION".into(), PROTO_CLI_VERSION.into()),
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

    inject_default_manifest_config(id, &proto.home, &mut manifest)?;
    inject_proto_manifest_config(id, proto, &mut manifest)?;

    ProtoTool::load_from_manifest(id, proto, manifest)
}
