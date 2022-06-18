use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptConfig {
    pub project_config_file_name: String,

    pub root_config_file_name: String,

    pub sync_project_references: bool,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            project_config_file_name: String::from("tsconfig.json"),
            root_config_file_name: String::from("tsconfig.json"),
            sync_project_references: true,
        }
    }
}
