use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate};

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct GoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl Default for GoConfig {
    fn default() -> Self {
        GoConfig {
            version: None,
        }
    }
}
