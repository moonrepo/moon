mod config_cache;
mod config_finder;
mod config_loader;
mod extensions_config_ext;
mod formats;
mod toolchains_config_ext;

pub use config_loader::*;
pub use extensions_config_ext::*;
use moon_common::is_test_env;
pub use toolchains_config_ext::*;

use proto_core::{PluginLocator, RegistryLocator, UrlLocator, warpgate::find_debug_locator};

pub fn find_debug_locator_with_fallback(name: &str, version: &str) -> PluginLocator {
    find_debug_locator(name).unwrap_or_else(|| {
        // Use URLs within tests, otherwise we hit ghcr.io rate limits when running
        // tests in CI without authentication. This causes tests to take forever!
        if is_test_env() {
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
