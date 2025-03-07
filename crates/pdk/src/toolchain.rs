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

/// Return true if the project has the current plugin/toolchain enabled.
pub fn is_project_toolchain_enabled(project: &ProjectFragment) -> bool {
    get_plugin_id().is_ok_and(|id| is_project_toolchain_enabled_for(project, id))
}

/// Return true if the project has the provided toolchain enabled.
pub fn is_project_toolchain_enabled_for(project: &ProjectFragment, id: impl AsRef<str>) -> bool {
    project.toolchains.contains(&Id::raw(id.as_ref()))
}
