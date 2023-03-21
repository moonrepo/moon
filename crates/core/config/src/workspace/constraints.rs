use crate::validators::{is_default, is_default_true};
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ConstraintsConfig {
    #[serde(skip_serializing_if = "is_default_true")]
    pub enforce_project_boundaries: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub tag_relationships: FxHashMap<String, Vec<String>>,
}

impl Default for ConstraintsConfig {
    fn default() -> Self {
        ConstraintsConfig {
            enforce_project_boundaries: true,
            tag_relationships: FxHashMap::default(),
        }
    }
}
