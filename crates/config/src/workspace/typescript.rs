use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptConfig {
    pub project_config_file_name: Option<String>,
    pub root_config_file_name: Option<String>,
    pub sync_project_references: Option<bool>,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            project_config_file_name: Some(String::from("tsconfig.json")),
            root_config_file_name: Some(String::from("tsconfig.json")),
            sync_project_references: Some(true),
        }
    }
}
