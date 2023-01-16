use crate::validators::is_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HasherOptimization {
    #[default]
    Accuracy,
    Performance,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct HasherConfig {
    #[serde(skip_serializing_if = "is_default")]
    pub optimization: HasherOptimization,
}
