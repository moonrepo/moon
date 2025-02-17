use extism_pdk::json;
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current toolchain plugin.
pub fn get_toolchain_config<T: Default + DeserializeOwned>(value: json::Value) -> AnyResult<T> {
    let config = json::from_value(value)?;

    // let config: T = if let Some(value) = config::get("moon_toolchain_config")? {
    //     json::from_str(&value)?
    // } else {
    //     T::default()
    // };

    Ok(config)
}
