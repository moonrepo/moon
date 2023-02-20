use crate::validators::{is_default, is_default_true};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct TypeScriptConfig {
    #[serde(skip_serializing_if = "is_default_true")]
    pub create_missing_config: bool,

    pub project_config_file_name: Option<String>,

    pub root_config_file_name: Option<String>,

    pub root_options_config_file_name: Option<String>,

    #[serde(skip_serializing_if = "is_default")]
    pub route_out_dir_to_cache: bool,

    #[serde(skip_serializing_if = "is_default_true")]
    pub sync_project_references: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub sync_project_references_to_paths: bool,
}

impl Default for TypeScriptConfig {
    fn default() -> Self {
        TypeScriptConfig {
            create_missing_config: true,
            project_config_file_name: None,
            root_config_file_name: None,
            root_options_config_file_name: None,
            route_out_dir_to_cache: false,
            sync_project_references: true,
            sync_project_references_to_paths: false,
        }
    }
}
