mod errors;
mod manager;
mod tool;

pub use async_trait::async_trait;
pub use errors::*;
pub use manager::*;
pub use tool::*;

use proto_core::{
    inject_default_manifest_config, Id, PluginLocator, ProtoEnvironment, Tool as ProtoTool,
    UserConfig, Wasm,
};
use std::env;
use std::path::Path;

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
    locator: &PluginLocator,
) -> miette::Result<ProtoTool> {
    let mut manifest = ProtoTool::create_plugin_manifest(
        proto,
        Wasm::file(proto.get_plugin_loader().load_plugin(id, locator).await?),
    )?;

    let user_config = UserConfig::load()?;

    inject_default_manifest_config(id, proto, &user_config, &mut manifest)?;

    ProtoTool::load_from_manifest(id, proto, manifest)
}
