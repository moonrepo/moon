use crate::validators::validate_semver_version;
use moon_lang_node::{NODENV, NVMRC, PNPM, YARN};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use validator::{Validate, ValidationError};

pub fn default_node_version() -> String {
    env::var("MOON_NODE_VERSION").unwrap_or_else(|_| String::from("16.15.0"))
}

pub fn default_npm_version() -> String {
    // Use the version bundled with node by default
    env::var("MOON_NPM_VERSION").unwrap_or_else(|_| String::from("inherit"))
}

pub fn default_pnpm_version() -> String {
    env::var("MOON_PNPM_VERSION").unwrap_or_else(|_| PNPM.default_version.to_string())
}

pub fn default_yarn_version() -> String {
    env::var("MOON_YARN_VERSION").unwrap_or_else(|_| YARN.default_version.to_string())
}

fn default_bool_true() -> bool {
    true
}

fn validate_node_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.version", value)
}

fn validate_npm_version(value: &str) -> Result<(), ValidationError> {
    if value != "inherit" {
        return validate_semver_version("node.npm.version", value);
    }

    Ok(())
}

fn validate_pnpm_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.pnpm.version", value)
}

fn validate_yarn_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("node.yarn.version", value)
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
}

impl Default for PackageManager {
    fn default() -> Self {
        PackageManager::Npm
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionManager {
    Nodenv,
    Nvm,
}

impl VersionManager {
    pub fn get_config_file_name(&self) -> String {
        match self {
            VersionManager::Nodenv => String::from(NODENV.version_filename),
            VersionManager::Nvm => String::from(NVMRC.version_filename),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct NpmConfig {
    #[serde(default = "default_npm_version")]
    #[validate(custom = "validate_npm_version")]
    pub version: String,
}

impl Default for NpmConfig {
    fn default() -> Self {
        NpmConfig {
            version: default_npm_version(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct PnpmConfig {
    #[serde(default = "default_pnpm_version")]
    #[validate(custom = "validate_pnpm_version")]
    pub version: String,
}

impl Default for PnpmConfig {
    fn default() -> Self {
        PnpmConfig {
            version: default_pnpm_version(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct YarnConfig {
    #[serde(default = "default_yarn_version")]
    #[validate(custom = "validate_yarn_version")]
    pub version: String,
}

impl Default for YarnConfig {
    fn default() -> Self {
        YarnConfig {
            version: default_yarn_version(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NodeConfig {
    #[serde(default = "default_bool_true")]
    pub add_engines_constraint: bool,

    #[serde(default = "default_bool_true")]
    pub dedupe_on_lockfile_change: bool,

    #[serde(default)]
    #[validate]
    pub npm: NpmConfig,

    #[serde(default)]
    pub package_manager: PackageManager,

    #[validate]
    pub pnpm: Option<PnpmConfig>,

    #[serde(default = "default_bool_true")]
    pub sync_project_workspace_dependencies: bool,

    pub sync_version_manager_config: Option<VersionManager>,

    #[serde(default = "default_node_version")]
    #[validate(custom = "validate_node_version")]
    pub version: String,

    #[validate]
    pub yarn: Option<YarnConfig>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            add_engines_constraint: default_bool_true(),
            dedupe_on_lockfile_change: default_bool_true(),
            npm: NpmConfig::default(),
            package_manager: PackageManager::Npm,
            pnpm: None,
            sync_project_workspace_dependencies: default_bool_true(),
            sync_version_manager_config: None,
            version: default_node_version(),
            yarn: None,
        }
    }
}
