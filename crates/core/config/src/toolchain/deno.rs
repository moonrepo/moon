use crate::validators::is_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoConfig {
    pub deps_file: String,

    #[serde(skip_serializing_if = "is_default")]
    pub lockfile: bool,
}

impl Default for DenoConfig {
    fn default() -> Self {
        DenoConfig {
            deps_file: "deps.ts".into(),
            lockfile: false,
        }
    }
}
