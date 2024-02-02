use extism_pdk::{config, json};
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current platform plugin.
pub fn get_platform_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("moon_platform_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}
