mod plugin;
mod plugin_registry;

pub use plugin::*;
pub use plugin_registry::*;

pub use extism::{Manifest as PluginManifest, Wasm};
pub use warpgate::{Id, PluginContainer, PluginLoader, PluginLocator};

use convert_case::{Case, Casing};
use miette::IntoDiagnostic;
use std::collections::BTreeMap;

pub fn serialize_config(
    base_config: &BTreeMap<String, serde_json::Value>,
) -> miette::Result<String> {
    let mut config = BTreeMap::new();

    for (key, value) in base_config {
        config.insert(
            if key.contains('-') || key.contains('_') {
                key.to_case(Case::Camel)
            } else {
                key.to_owned()
            },
            value.to_owned(),
        );
    }

    serde_json::to_string(&config).into_diagnostic()
}
