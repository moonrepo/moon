use crate::validators::{is_default, is_default_true};
use moon_common::Id;
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstraintsConfig {
    #[serde(skip_serializing_if = "is_default_true")]
    pub enforce_project_type_relationships: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub tag_relationships: FxHashMap<Id, Vec<Id>>,
}

impl Default for ConstraintsConfig {
    fn default() -> Self {
        ConstraintsConfig {
            enforce_project_type_relationships: true,
            tag_relationships: FxHashMap::default(),
        }
    }
}
