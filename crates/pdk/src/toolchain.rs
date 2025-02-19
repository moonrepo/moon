use extism_pdk::json;
use moon_common::Id;
use moon_project::ProjectFragment;
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current toolchain plugin.
pub fn get_toolchain_config<T: Default + DeserializeOwned>(value: json::Value) -> AnyResult<T> {
    let config = json::from_value(value)?;

    Ok(config)
}

/// Return true if the project has the provided toolchain enabled.
pub fn is_project_toolchain_enabled(project: &ProjectFragment, id: impl AsRef<str>) -> bool {
    project.toolchains.contains(&Id::raw(id.as_ref()))
}
