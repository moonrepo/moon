use crate::validators::validate_semver_version;
use serde::{Deserialize, Serialize};
use std::env;
use validator::{Validate, ValidationError};

const NODE_VERSION: &str = "16.14.0";
const NPM_VERSION: &str = "inherit"; // Use the version bundled with node
const PNPM_VERSION: &str = "6.32.2";
const YARN_VERSION: &str = "3.2.0";

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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Pnpm,
    Yarn,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct NpmConfig {
    #[validate(custom = "validate_npm_version")]
    pub version: String,
}

impl Default for NpmConfig {
    fn default() -> Self {
        NpmConfig {
            version: env::var("MOON_NPM_VERSION").unwrap_or_else(|_| NPM_VERSION.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct PnpmConfig {
    #[validate(custom = "validate_pnpm_version")]
    pub version: String,
}

impl Default for PnpmConfig {
    fn default() -> Self {
        PnpmConfig {
            version: env::var("MOON_PNPM_VERSION").unwrap_or_else(|_| PNPM_VERSION.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct YarnConfig {
    #[validate(custom = "validate_yarn_version")]
    pub version: String,
}

impl Default for YarnConfig {
    fn default() -> Self {
        YarnConfig {
            version: env::var("MOON_YARN_VERSION").unwrap_or_else(|_| YARN_VERSION.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct NodeConfig {
    pub dedupe_on_install: Option<bool>,

    #[validate]
    pub npm: Option<NpmConfig>,

    pub package_manager: Option<PackageManager>,

    #[validate]
    pub pnpm: Option<PnpmConfig>,

    pub sync_project_workspace_dependencies: Option<bool>,

    #[validate(custom = "validate_node_version")]
    pub version: String,

    #[validate]
    pub yarn: Option<YarnConfig>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            dedupe_on_install: Some(true),
            npm: Some(NpmConfig::default()),
            package_manager: Some(PackageManager::Npm),
            pnpm: None,
            sync_project_workspace_dependencies: Some(true),
            version: env::var("MOON_NODE_VERSION").unwrap_or_else(|_| NODE_VERSION.to_owned()),
            yarn: None,
        }
    }
}
