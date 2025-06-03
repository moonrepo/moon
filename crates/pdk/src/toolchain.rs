use extism_pdk::{config, json};
use moon_common::Id;
use moon_project::ProjectFragment;
use serde::de::DeserializeOwned;
use warpgate_pdk::{AnyResult, get_plugin_id};

/// Get workspace-level configuration for the current toolchain plugin.
pub fn get_toolchain_config<T: Default + DeserializeOwned>() -> AnyResult<T> {
    let config: T = if let Some(value) = config::get("moon_toolchain_config")? {
        json::from_str(&value)?
    } else {
        T::default()
    };

    Ok(config)
}

/// Parse workspace & project merged configuration for the current toolchain plugin.
pub fn parse_toolchain_config<T: Default + DeserializeOwned>(value: json::Value) -> AnyResult<T> {
    let config = if value.is_object() {
        json::from_value(value)?
    } else {
        T::default()
    };

    Ok(config)
}

/// Parse workspace & project merged configuration for the current toolchain plugin
/// using `schematic`. Will run any validation rules.
#[cfg(feature = "schematic")]
pub fn parse_toolchain_config_schema<T: schematic::Config>(value: json::Value) -> AnyResult<T> {
    use moon_pdk_api::anyhow;
    use schematic::{ConfigLoader, Format};

    match ConfigLoader::<T>::new()
        .code(json::to_string(&value)?, Format::Json)?
        .load()
    {
        Ok(result) => Ok(result.config),

        // miette swallows a bunch of error information since the errors are nested,
        // so we must convert to a string to extract all the relevant information
        Err(error) => Err(anyhow!("{}", error.to_full_string())),
    }
}

/// Return true if the project has the current plugin/toolchain enabled.
pub fn is_project_toolchain_enabled(project: &ProjectFragment) -> bool {
    get_plugin_id().is_ok_and(|id| is_project_toolchain_enabled_for(project, id))
}

/// Return true if the project has the provided toolchain enabled.
pub fn is_project_toolchain_enabled_for(project: &ProjectFragment, id: impl AsRef<str>) -> bool {
    project.toolchains.contains(&Id::raw(id.as_ref()))
}
