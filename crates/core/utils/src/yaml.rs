use lazy_static::lazy_static;
use moon_error::MoonError;
use moon_error::{map_io_to_fs_error, map_yaml_to_error};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::Path;

pub use serde_yaml::{Mapping, Value as YamlValue};

lazy_static! {
    static ref LINE_WS_START: Regex = Regex::new(r"^(\s+)").unwrap();
}

#[inline]
pub fn merge(prev: &YamlValue, next: &YamlValue) -> YamlValue {
    match (prev, next) {
        (YamlValue::Mapping(prev_object), YamlValue::Mapping(next_object)) => {
            let mut object = prev_object.clone();

            for (key, value) in next_object.iter() {
                if let Some(prev_value) = prev_object.get(key) {
                    object.insert(key.to_owned(), merge(prev_value, value));
                } else {
                    object.insert(key.to_owned(), value.to_owned());
                }
            }

            YamlValue::Mapping(object)
        }
        _ => next.to_owned(),
    }
}

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
    let path = path.as_ref();
    let editor_config = crate::fs::get_editor_config_props(path);

    let mut data = serde_yaml::to_string(&yaml)
        .map_err(|e| map_yaml_to_error(e, path.to_path_buf()))?
        .trim()
        .to_string();

    // serde_yaml does not support customizing the indentation character. So to work around
    // this, we do it manually on the YAML string, but only if the indent is different than
    // a double space (the default). Can be customized with `.editorconfig`.
    if editor_config.indent != "  " {
        data = data
            .split('\n')
            .map(|line| {
                if !line.starts_with("  ") {
                    return line.to_string();
                }

                LINE_WS_START
                    .replace_all(line, |caps: &regex::Captures| {
                        editor_config
                            .indent
                            .repeat(caps.get(1).unwrap().as_str().len() / 2)
                    })
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    data += &editor_config.eof;

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}
