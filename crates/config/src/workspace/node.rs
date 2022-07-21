use crate::validators::validate_semver_version;
use moon_lang_node::{NODE, NODENV, NPM, NVMRC, PNPM, YARN};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::env;
use validator::{Validate, ValidationError};

pub fn default_node_version() -> String {
    env::var("MOON_NODE_VERSION").unwrap_or_else(|_| NODE.default_version.to_string())
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

impl PackageManager {
    pub fn get_bin_name(&self) -> String {
        match self {
            PackageManager::Npm => NPM.binary.to_owned(),
            PackageManager::Pnpm => PNPM.binary.to_owned(),
            PackageManager::Yarn => YARN.binary.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VersionManager {
    Nodenv,
    Nvm,
}

impl VersionManager {
    pub fn get_config_filename(&self) -> String {
        match self {
            VersionManager::Nodenv => String::from(NODENV.version_filename),
            VersionManager::Nvm => String::from(NVMRC.version_filename),
        }
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
pub struct NpmConfig {
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
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct NodeConfig {
    pub add_engines_constraint: bool,

    pub dedupe_on_lockfile_change: bool,

    pub infer_tasks_from_scripts: bool,

    #[validate]
    pub npm: NpmConfig,

    pub package_manager: PackageManager,

    #[validate]
    pub pnpm: Option<PnpmConfig>,

    pub sync_project_workspace_dependencies: bool,

    pub sync_version_manager_config: Option<VersionManager>,

    #[validate(custom = "validate_node_version")]
    pub version: String,

    #[validate]
    pub yarn: Option<YarnConfig>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            add_engines_constraint: true,
            dedupe_on_lockfile_change: true,
            infer_tasks_from_scripts: false,
            npm: NpmConfig::default(),
            package_manager: PackageManager::default(),
            pnpm: None,
            sync_project_workspace_dependencies: true,
            sync_version_manager_config: None,
            version: default_node_version(),
            yarn: None,
        }
    }
}
