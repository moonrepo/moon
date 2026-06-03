mod config_cache;
mod config_finder;
mod config_loader;
mod extensions_config_ext;
mod formats;
mod toolchains_config_ext;

pub use config_loader::*;
pub use extensions_config_ext::*;
pub use toolchains_config_ext::*;

use proto_core::{PluginLocator, RegistryLocator, UrlLocator, warpgate::find_debug_locator};
use starbase_utils::envx::bool_var;
use std::sync::OnceLock;

pub fn find_debug_locator_with_fallback(name: &str, version: &str) -> PluginLocator {
    static URL_CACHE: OnceLock<bool> = OnceLock::new();

    let use_urls = *URL_CACHE.get_or_init(|| bool_var("MOON_PLUGINS_USE_URL_DIST"));

    find_debug_locator(name).unwrap_or_else(|| {
        if use_urls {
            PluginLocator::Url(Box::new(UrlLocator {
                url: format!(
                    "https://github.com/moonrepo/plugins/releases/download/{name}-v{version}/{name}.wasm"
                ),
            }))
        } else {
            PluginLocator::Registry(Box::new(RegistryLocator {
                registry: Some("ghcr.io".into()),
                namespace: Some("moonrepo".into()),
                image: name.into(),
                tag: Some(version.into()),
            }))
        }
    })
}
