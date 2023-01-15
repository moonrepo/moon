// These configs are project-level settings that override those from the root!

use crate::validators::validate_semver_version;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_node_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("toolchain.node.version", value)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct ProjectToolchainNodeConfig {
    #[validate(custom = "validate_node_version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectToolchainConfig {
    #[validate]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<ProjectToolchainNodeConfig>,

    pub typescript: bool,
}

impl Default for ProjectToolchainConfig {
    fn default() -> Self {
        ProjectToolchainConfig {
            node: None,
            typescript: true,
        }
    }
}
