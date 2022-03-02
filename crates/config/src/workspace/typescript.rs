use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptConfig {
    pub sync_project_references: Option<bool>,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            sync_project_references: Some(true),
        }
    }
}
