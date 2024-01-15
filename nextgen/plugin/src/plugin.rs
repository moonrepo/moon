use extism::{Manifest as PluginManifest, Wasm};
use std::collections::BTreeMap;
use std::path::PathBuf;
use warpgate::Id;

#[derive(Clone, Copy, Debug)]
pub enum PluginType {
    Extension,
    Platform,
}

pub trait Plugin
where
    Self: Sized,
{
    fn new(id: Id, wasm_file: PathBuf) -> miette::Result<Self>;
    fn get_type(&self) -> PluginType;
}

pub fn create_plugin_manifest(
    wasm_file: PathBuf,
    virtual_paths: BTreeMap<PathBuf, PathBuf>,
) -> PluginManifest {
    let mut manifest = PluginManifest::new([Wasm::file(wasm_file)]);

    // Allow all hosts because we don't know what endpoints plugins
    // will communicate with. Far too many to account for.
    manifest = manifest.with_allowed_host("*");

    // Inherit moon and proto virtual paths.
    manifest = manifest.with_allowed_paths(virtual_paths.into_iter());

    // Disable timeouts as some functions, like dependency installs,
    // may take multiple minutes to complete. We also can't account
    // for network connectivity.
    manifest.timeout_ms = None;

    manifest
}
