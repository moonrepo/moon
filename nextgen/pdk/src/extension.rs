use extism_pdk::{config, json};
use serde::de::DeserializeOwned;
use warpgate_api::pdk::AnyResult;

/// Get configuration for the current extension plugin.
pub fn get_extension_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("moon_extension_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}
