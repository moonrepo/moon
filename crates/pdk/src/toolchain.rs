use extism_pdk::json;
use moon_common::Id;
use moon_project::Project;
use serde::de::DeserializeOwned;
use warpgate_pdk::AnyResult;

/// Get configuration for the current toolchain plugin.
pub fn get_toolchain_config<T: Default + DeserializeOwned>(value: json::Value) -> AnyResult<T> {
    let config = json::from_value(value)?;

    Ok(config)
}

/// Return true if the project has the provided toolchain enabled.
pub fn is_project_toolchain_enabled(project: &Project, id: &str) -> bool {
    let id = Id::raw(id);

    if !project.toolchains.contains(&id) {
        return false;
    }

    match project.config.toolchain.toolchains.get(&id) {
        None => true,
        Some(cfg) => cfg.is_enabled(),
    }
}
