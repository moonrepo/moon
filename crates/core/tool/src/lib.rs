mod errors;
mod manager;
mod tool;

pub use async_trait::async_trait;
pub use errors::*;
pub use manager::*;
pub use tool::*;

use proto_core::{
    inject_default_manifest_config, Id, PluginLoader, PluginLocator, ProtoEnvironment,
    Tool as ProtoTool, UserConfig, Wasm,
};
use std::env;
use std::path::Path;
use std::str::FromStr;

/// We need to ensure that our toolchain binaries are executed instead of
/// other binaries of the same name. Otherwise, tooling like nvm will
/// intercept execution and break our processes. We can work around this
/// by prepending the `PATH` environment variable.
pub fn get_path_env_var(bin_dir: &Path) -> std::ffi::OsString {
    let path = env::var("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir.to_path_buf()];

    paths.extend(env::split_paths(&path).collect::<Vec<_>>());

    env::join_paths(paths).unwrap()
}

pub async fn load_tool_plugin(
    id: &Id,
    proto: &ProtoEnvironment,
    locator: &str,
    loader: &PluginLoader,
) -> miette::Result<ProtoTool> {
    let locator = PluginLocator::from_str(locator)?;

    let mut manifest = ProtoTool::create_plugin_manifest(
        proto,
        Wasm::file(loader.load_plugin(id, locator).await?),
    )?;

    let user_config = UserConfig::load()?;
    let mut config = std::collections::HashMap::new();

    inject_default_manifest_config(id, proto, &user_config, &manifest, &mut config)?;

    manifest.config.extend(config);

    ProtoTool::load_from_manifest(id, proto, manifest)
}
