use cached::proc_macro::cached;
use moon_lang::config_cache_container;
use serde::{Deserialize, Serialize};
use starbase_utils::toml::{read_file as read_toml, write_file};
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
pub struct ToolchainSection {
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolchainToml {
    pub toolchain: ToolchainSection,
}

impl ToolchainToml {
    pub fn new_with_channel<T: AsRef<str>>(channel: T) -> ToolchainToml {
        ToolchainToml {
            toolchain: ToolchainSection {
                channel: Some(channel.as_ref().to_owned()),
                ..ToolchainSection::default()
            },
        }
    }
}

pub fn write_toml(path: &Path, toml: &ToolchainToml) -> miette::Result<()> {
    write_file(path, toml, true)?;

    Ok(())
}

config_cache_container!(
    ToolchainTomlCache,
    ToolchainToml,
    "rust-toolchain.toml",
    read_toml,
    write_toml
);
