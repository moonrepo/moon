//! Prototype-only loader for the local Jujutsu guest.

use moon_plugin::{MoonEnvironment, MoonHostData, ProtoEnvironment};
use moon_vcs_plugin::load_verified_vcs_plugin;
use starbase_utils::hash;
use std::path::Path;
use std::sync::Arc;
use warpgate::{FileLocator, Id, PluginLocator};

pub async fn load_prototype_plugin(
    moon_root: &Path,
    repository_root: &Path,
) -> miette::Result<Arc<VcsPlugin>> {
    let wasm_file = moon_root.join("wasm/target/wasm32-wasip1/release/vcs_jj_prototype.wasm");
    let expected_sha256 = hash::sha256::from_file(&wasm_file)?;

    load_verified_prototype_plugin(
        repository_root,
        PluginLocator::File(Box::new(FileLocator {
            file: String::new(),
            path: Some(wasm_file),
        })),
        &expected_sha256,
    )
    .await
}

pub async fn load_verified_prototype_plugin(
    repository_root: &Path,
    locator: PluginLocator,
    expected_sha256: &str,
) -> miette::Result<Arc<VcsPlugin>> {
    let mut moon_env = MoonEnvironment::new()?;
    moon_env.working_dir = repository_root.to_path_buf();
    moon_env.workspace_root = repository_root.to_path_buf();

    let mut proto_env = ProtoEnvironment::new()?;
    proto_env.working_dir = repository_root.to_path_buf();

    load_verified_vcs_plugin(
        MoonHostData {
            moon_env: Arc::new(moon_env),
            proto_env: Arc::new(proto_env),
            ..Default::default()
        },
        Id::raw("jj-prototype"),
        locator,
        expected_sha256,
    )
    .await
}

pub use moon_vcs_plugin::VcsPlugin;
