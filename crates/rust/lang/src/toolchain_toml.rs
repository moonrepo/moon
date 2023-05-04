use crate::RUSTUP;
use cached::proc_macro::cached;
use moon_error::MoonError;
use moon_lang::config_cache_container;
use serde::{Deserialize, Serialize};
use starbase_utils::toml::{read_file as read_toml, write_file, TomlError};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolchainProfile {
    Minimal,
    #[default]
    Default,
    Complete,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolchainToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ToolchainProfile>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub targets: Option<Vec<String>>,
}

pub fn write_toml(path: &Path, toml: &ToolchainToml) -> Result<(), TomlError> {
    write_file(path, toml, true)
}

config_cache_container!(
    ToolchainTomlCache,
    ToolchainToml,
    RUSTUP.version_file,
    read_toml,
    write_toml
);
