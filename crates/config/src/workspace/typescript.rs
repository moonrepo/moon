use serde::{Deserialize, Serialize};
use validator::Validate;

fn default_config_file_name() -> String {
    String::from("tsconfig.json")
}

fn default_sync_project_references() -> bool {
    true
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct TypeScriptConfig {
    #[serde(default = "default_config_file_name")]
    pub project_config_file_name: String,

    #[serde(default = "default_config_file_name")]
    pub root_config_file_name: String,

    #[serde(default = "default_sync_project_references")]
    pub sync_project_references: bool,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            project_config_file_name: default_config_file_name(),
            root_config_file_name: default_config_file_name(),
            sync_project_references: default_sync_project_references(),
        }
    }
}
