use crate::validators::{is_default, validate_semver_version};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_rust_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("rust.version", value)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct RustConfig {
    #[serde(skip_serializing_if = "is_default")]
    pub sync_toolchain_config: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_rust_version")]
    pub version: Option<String>,
}
