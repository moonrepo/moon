use extism::{Manifest as PluginManifest, Wasm};
use proto_core::ProtoEnvironment;

pub enum PluginType {
    Extension,
    Platform,
}

pub trait Plugin {
    fn get_type(&self) -> PluginType;
}

pub fn create_plugin_manifest<P: AsRef<ProtoEnvironment>>(
    proto: P,
    wasm: Wasm,
) -> miette::Result<PluginManifest> {
    let proto = proto.as_ref();

    let mut manifest = PluginManifest::new([wasm]);
    manifest = manifest.with_allowed_host("*");
    manifest = manifest.with_allowed_paths(proto.get_virtual_paths().into_iter());
    manifest = manifest.with_timeout(Duration::from_secs(90));

    #[cfg(debug_assertions)]
    {
        manifest = manifest.with_timeout(Duration::from_secs(120));
    }

    Ok(manifest)
}
