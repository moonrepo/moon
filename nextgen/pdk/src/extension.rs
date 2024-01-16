use extism_pdk::*;
use serde::de::DeserializeOwned;

/// Get configuration for the current extension plugin.
pub fn get_extension_config<T: Default + DeserializeOwned>() -> anyhow::Result<T> {
    let config: T = if let Some(value) = config::get("moon_extension_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}
