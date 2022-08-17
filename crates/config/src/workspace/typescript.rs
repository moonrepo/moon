use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct TypeScriptConfig {
    pub create_missing_config: bool,

    pub project_config_file_name: String,

    pub root_config_file_name: String,

    pub root_options_config_file_name: String,

    pub sync_project_references: bool,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            create_missing_config: true,
            project_config_file_name: String::from("tsconfig.json"),
            root_config_file_name: String::from("tsconfig.json"),
            root_options_config_file_name: String::from("tsconfig.options.json"),
            sync_project_references: true,
        }
    }
}
