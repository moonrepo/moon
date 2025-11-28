use extism_pdk::{config, json};
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current extension plugin.
pub fn get_extension_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("moon_extension_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}

/// Parse configuration for the current extension plugin.
pub fn parse_extension_config<T: Default + DeserializeOwned>(value: json::Value) -> AnyResult<T> {
    let config = if value.is_object() {
        json::from_value(value)?
    } else {
        T::default()
    };

    Ok(config)
}

/// Parse workspace & project merged configuration for the current extension plugin
/// using `schematic`. Will run any validation rules.
#[cfg(feature = "schematic")]
pub fn parse_extension_config_schema<T: schematic::Config>(value: json::Value) -> AnyResult<T> {
    use moon_pdk_api::anyhow;
    use schematic::{ConfigLoader, Format};

    match ConfigLoader::<T>::new()
        .code(
            match value {
                json::Value::Null => "{}".to_owned(),
                _ => json::to_string(&value)?,
            },
            Format::Json,
        )?
        .load()
    {
        Ok(result) => Ok(result.config),

        // miette swallows a bunch of error information since the errors are nested,
        // so we must convert to a string to extract all the relevant information
        Err(error) => Err(anyhow!("{}", error.to_full_string())),
    }
}
