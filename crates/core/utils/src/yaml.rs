use moon_error::{map_io_to_fs_error, map_yaml_to_error, MoonError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

// pub use yaml_rust::{
//     yaml::{Array, Hash},
//     Yaml, YamlEmitter, YamlLoader,
// };

pub use serde_yaml::Value as YamlValue;

#[inline]
pub fn read<P, D>(path: P) -> Result<D, MoonError>
where
    P: AsRef<Path>,
    D: DeserializeOwned,
{
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    serde_yaml::from_str(&contents).map_err(|e| map_yaml_to_error(e, path.to_path_buf()))
}

#[inline]
pub fn write<P, D>(path: P, yaml: &D) -> Result<(), MoonError>
where
    P: AsRef<Path>,
    D: ?Sized + Serialize,
{
    let path = path.as_ref();
    let data =
        serde_yaml::to_string(&yaml).map_err(|e| map_yaml_to_error(e, path.to_path_buf()))?;

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}

// This function is used for consumer facing files, like configs.
#[inline]
pub fn write_with_config<P: AsRef<Path>>(path: P, yaml: YamlValue) -> Result<(), MoonError> {
    write(path, &yaml)
}
