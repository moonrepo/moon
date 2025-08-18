use extism_pdk::{config, json};
use moon_pdk_api::VirtualPath;
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

/// Return true if the project has the current plugin/toolchain enabled.
pub fn is_project_toolchain_enabled(project: &ProjectFragment) -> bool {
    get_plugin_id().is_ok_and(|id| is_project_toolchain_enabled_for(project, id))
}

/// Return true if the project has the provided toolchain enabled.
pub fn is_project_toolchain_enabled_for(project: &ProjectFragment, id: impl AsRef<str>) -> bool {
    project
        .toolchains
        .iter()
        .any(|tc| tc.as_str() == id.as_ref())
}

/// Locate the root directory that contains the provided file name,
/// by traversing upwards from the starting directory.
pub fn locate_root(starting_dir: &VirtualPath, file_name: &str) -> Option<VirtualPath> {
    locate_root_many(starting_dir, &[file_name])
}

/// Locate the root directory that contains the provided file name,
/// by traversing upwards from the starting directory and running
/// the check function on each directory. If the check returns true,
/// the traversal will stop.
pub fn locate_root_with_check(
    starting_dir: &VirtualPath,
    file_name: &str,
    check: impl FnMut(&VirtualPath) -> AnyResult<bool>,
) -> AnyResult<()> {
    locate_root_many_with_check(starting_dir, &[file_name], check)
}

/// Locate the root directory that contains the provided file name(s),
/// by traversing upwards from the starting directory.
pub fn locate_root_many<T: AsRef<str>>(
    starting_dir: &VirtualPath,
    file_names: &[T],
) -> Option<VirtualPath> {
    let mut current_dir = Some(starting_dir.to_owned());

    while let Some(dir) = current_dir {
        for file_name in file_names {
            if dir.join(file_name.as_ref()).exists() {
                return Some(dir);
            }
        }

        current_dir = dir.parent();
    }

    None
}

/// Locate the root directory that contains the provided file name(s),
/// by traversing upwards from the starting directory and running
/// the check function on each directory. If the check returns true,
/// the traversal will stop.
pub fn locate_root_many_with_check<T: AsRef<str>>(
    starting_dir: &VirtualPath,
    file_names: &[T],
    mut check: impl FnMut(&VirtualPath) -> AnyResult<bool>,
) -> AnyResult<()> {
    let mut current_dir = Some(starting_dir.to_owned());

    while let Some(dir) = current_dir {
        for file_name in file_names {
            if dir.join(file_name.as_ref()).exists() && check(&dir)? {
                return Ok(());
            }
        }

        current_dir = dir.parent();
    }

    Ok(())
}
