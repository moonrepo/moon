use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct DenoConfig {
    pub deps_file: String,

    pub lock_file: String,
}

impl Default for DenoConfig {
    fn default() -> Self {
        DenoConfig {
            deps_file: "src/deps.ts".into(),
            lock_file: "deno.lock".into(),
        }
    }
}
