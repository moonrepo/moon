mod config_cache;
mod config_finder;
mod config_loader;
mod extensions_config_ext;
mod formats;
mod toolchains_config_ext;

pub use config_loader::*;
pub use extensions_config_ext::*;
pub use toolchains_config_ext::*;

use proto_core::{PluginLocator, RegistryLocator, warpgate::find_debug_locator};

pub fn find_debug_locator_with_fallback(name: &str, version: &str) -> PluginLocator {
    find_debug_locator(name).unwrap_or_else(|| {
        PluginLocator::Registry(Box::new(RegistryLocator {
            registry: Some("ghcr.io".into()),
            namespace: Some("moonrepo".into()),
            image: name.into(),
            tag: Some(version.into()),
        }))
    })
}
