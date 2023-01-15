use crate::validators::is_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct TypeScriptConfig {
    #[serde(skip_serializing_if = "is_default")]
    pub create_missing_config: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub project_config_file_name: String,

    #[serde(skip_serializing_if = "is_default")]
    pub root_config_file_name: String,

    #[serde(skip_serializing_if = "is_default")]
    pub root_options_config_file_name: String,

    #[serde(skip_serializing_if = "is_default")]
    pub route_out_dir_to_cache: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub sync_project_references: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub sync_project_references_to_paths: bool,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            create_missing_config: true,
            project_config_file_name: "tsconfig.json".into(),
            root_config_file_name: "tsconfig.json".into(),
            root_options_config_file_name: "tsconfig.options.json".into(),
            route_out_dir_to_cache: false,
            sync_project_references: true,
            sync_project_references_to_paths: false,
        }
    }
}
