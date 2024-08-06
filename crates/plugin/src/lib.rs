mod plugin;
mod plugin_error;
mod plugin_registry;

pub use moon_env::MoonEnvironment;
pub use plugin::*;
pub use plugin_error::*;
pub use plugin_registry::*;
pub use proto_core::ProtoEnvironment;
pub use warpgate::{
    Id as PluginId, PluginContainer, PluginLoader, PluginLocator, PluginManifest, Wasm,
};

use convert_case::{Case, Casing};
use miette::IntoDiagnostic;
use std::collections::BTreeMap;

pub fn serialize_config<'cfg>(
    base_config: impl Iterator<Item = (&'cfg String, &'cfg serde_json::Value)>,
) -> miette::Result<String> {
    let mut config = BTreeMap::new();

    for (key, value) in base_config {
        config.insert(
            if key.contains('-') || key.contains('_') {
                key.to_case(Case::Camel)
            } else {
                key.to_owned()
            },
            value,
        );
    }

    serde_json::to_string(&config).into_diagnostic()
}
