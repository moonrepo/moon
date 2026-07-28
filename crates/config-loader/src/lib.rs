mod config_cache;
mod config_finder;
mod config_loader;
mod extensions_config_ext;
mod formats;
mod toolchains_config_ext;

pub use config_loader::*;
pub use extensions_config_ext::*;
pub use toolchains_config_ext::*;

use miette::IntoDiagnostic;
use proto_core::{PluginLocator, RegistryLocator, UrlLocator, warpgate::find_debug_locator};
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_utils::envx::bool_var;
use starbase_utils::{fs, json, toml, yaml};
use std::path::Path;
use std::sync::OnceLock;

pub fn read_config_based_on_extension<T: DeserializeOwned>(path: &Path) -> miette::Result<T> {
    let config: T = match path.extension().and_then(|ext| ext.to_str()) {
        Some("hcl") => {
            let content = fs::read_file(path)?;
            hcl::from_str(&content).into_diagnostic()?
        }
        Some("json" | "jsonc") => json::read_file(path)?,
        Some("toml") => toml::read_file(path)?,
        Some("yml" | "yaml") => yaml::read_file(path)?,
        _ => unimplemented!(),
    };

    Ok(config)
}

pub fn write_config_based_on_extension<T: Serialize>(path: &Path, config: T) -> miette::Result<()> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("hcl") => {
            let content = hcl::to_string(&config).into_diagnostic()?;
            fs::write_file(path, &content)?;
        }
        Some("json" | "jsonc") => json::write_file(path, &config, true)?,
        Some("toml") => toml::write_file(path, &config, true)?,
        Some("yml" | "yaml") => yaml::write_file(path, &config)?,
        _ => unimplemented!(),
    };

    Ok(())
}

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
