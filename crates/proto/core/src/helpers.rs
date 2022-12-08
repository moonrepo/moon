use crate::ProtoError;
use dirs::home_dir;
use std::{env, path::PathBuf};

pub fn get_dir() -> Result<PathBuf, ProtoError> {
    if let Ok(dir) = env::var("PROTO_DIR") {
        return Ok(dir.into());
    }

    if let Some(dir) = home_dir() {
        return Ok(dir.join(".proto"));
    }

    Err(ProtoError::MissingHomeDir)
}

pub fn get_temp_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_dir()?.join("temp"))
}

pub fn get_tools_dir() -> Result<PathBuf, ProtoError> {
    Ok(get_dir()?.join("tools"))
}

// Aliases are words that map to version. For example, "latest" -> "1.2.3".
pub fn is_version_alias(value: &str) -> bool {
    value
        .chars()
        .all(|c| char::is_ascii_alphabetic(&c) || c == '-')
}

pub fn add_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value.to_lowercase();
    }

    format!("v{}", value)
}

pub fn remove_v_prefix(value: &str) -> String {
    if value.starts_with('v') || value.starts_with('V') {
        return value[1..].to_owned();
    }

    value.to_owned()
}
