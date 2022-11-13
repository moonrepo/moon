use ec4rs::property::*;
use json_comments::StripComments;
use moon_error::{map_io_to_fs_error, map_json_to_error, MoonError};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::Path;

pub use json::{from, parse, JsonValue};

#[inline]
pub fn clean<D: AsRef<str>>(json: D) -> Result<String, MoonError> {
    let json = json.as_ref();

    // Remove comments
    let mut stripped = String::with_capacity(json.len());

    StripComments::new(json.as_bytes())
        .read_to_string(&mut stripped)
        .map_err(MoonError::Unknown)?;

    // Remove trailing commas
    let stripped = Regex::new(r",(?P<valid>\s*})")
        .unwrap()
        .replace_all(&stripped, "$valid");

    Ok(String::from(stripped))
}

#[inline]
pub fn merge(prev: &JsonValue, next: &JsonValue) -> JsonValue {
    match (prev, next) {
        (JsonValue::Object(prev_object), JsonValue::Object(next_object)) => {
            let mut object = prev_object.clone();

            for (key, value) in next_object.iter() {
                if let Some(prev_value) = prev_object.get(key) {
                    object.insert(key, merge(prev_value, value));
                } else {
                    object.insert(key, value.to_owned());
                }
            }

            JsonValue::Object(object)
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
    let contents = read_to_string(path)?;

    let json: D =
        serde_json::from_str(&contents).map_err(|e| map_json_to_error(e, path.to_path_buf()))?;

    Ok(json)
}

#[inline]
pub fn read_raw<T: AsRef<Path>>(path: T) -> Result<JsonValue, MoonError> {
    let path = path.as_ref();
    let data = read_to_string(path)?;

    parse(&data).map_err(|e| MoonError::Generic(e.to_string()))
}

#[inline]
pub fn read_to_string<T: AsRef<Path>>(path: T) -> Result<String, MoonError> {
    let path = path.as_ref();
    let data = fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    clean(data)
}

// This function is primarily used internally for non-consumer facing files.
#[inline]
pub fn write<P, D>(path: P, json: &D, pretty: bool) -> Result<(), MoonError>
where
    P: AsRef<Path>,
    D: ?Sized + Serialize,
{
    let path = path.as_ref();
    let data = if pretty {
        serde_json::to_string_pretty(&json).map_err(|e| map_json_to_error(e, path.to_path_buf()))?
    } else {
        serde_json::to_string(&json).map_err(|e| map_json_to_error(e, path.to_path_buf()))?
    };

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}

// This function is used for consumer facing files, like configs.
#[inline]
pub fn write_raw<P: AsRef<Path>>(path: P, json: JsonValue, pretty: bool) -> Result<(), MoonError> {
    let path = path.as_ref();

    if !pretty {
        fs::write(path, json::stringify(json))
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

        return Ok(());
    }

    let editor_config = ec4rs::properties_of(path).unwrap_or_default();
    let indent_size = editor_config
        .get::<IndentSize>()
        .unwrap_or(IndentSize::Value(2));
    let insert_final_newline = editor_config
        .get::<FinalNewline>()
        .unwrap_or(FinalNewline::Value(true));

    // json crate doesnt support tabs, so always use space indentation
    let spaces = match indent_size {
        IndentSize::UseTabWidth => 2,
        IndentSize::Value(value) => value,
    };

    let mut data = json::stringify_pretty(json, spaces as u16);

    if matches!(insert_final_newline, FinalNewline::Value(true)) {
        data += "\n";
    }

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}
