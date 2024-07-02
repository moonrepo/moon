use extism_pdk::{config, json};
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current toolchain plugin.
pub fn get_toolchain_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("moon_toolchain_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}
