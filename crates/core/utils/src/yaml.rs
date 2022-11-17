use moon_error::{map_io_to_fs_error, map_yaml_to_error, MoonError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub use yaml_rust::{
    yaml::{Array, Hash},
    Yaml, YamlEmitter, YamlLoader,
};

#[inline]
pub fn read<P, D>(path: P) -> Result<D, MoonError>
where
    P: AsRef<Path>,
    D: DeserializeOwned,
{
    let path = path.as_ref();
    let contents =
        fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    let yaml: D =
        serde_yaml::from_str(&contents).map_err(|e| map_yaml_to_error(e, path.to_path_buf()))?;

    Ok(yaml)
}

#[inline]
pub fn read_raw<T: AsRef<Path>>(path: T) -> Result<Yaml, MoonError> {
    let path = path.as_ref();
    let data = fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    let docs = YamlLoader::load_from_str(&data).map_err(|e| MoonError::Generic(e.to_string()))?;

    docs.into_iter()
        .next()
        .ok_or_else(|| MoonError::Generic("Invalid YAML document.".into()))
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
pub fn write_raw<P: AsRef<Path>>(path: P, yaml: Yaml) -> Result<(), MoonError> {
    let path = path.as_ref();
    let mut data = String::new();
    let mut emitter = YamlEmitter::new(&mut data);

    emitter
        .dump(&yaml)
        .map_err(|e| MoonError::Generic(e.to_string()))?;

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}
