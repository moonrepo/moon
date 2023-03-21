use crate::validators::{is_default, is_default_true};
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

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HasherWalkStrategy {
    Glob,
    #[default]
    Vcs,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct HasherConfig {
    #[serde(skip_serializing_if = "is_default")]
    pub optimization: HasherOptimization,

    #[serde(skip_serializing_if = "is_default")]
    pub walk_strategy: HasherWalkStrategy,

    #[serde(skip_serializing_if = "is_default_true")]
    pub warn_on_missing_inputs: bool,
}

impl Default for HasherConfig {
    fn default() -> Self {
        HasherConfig {
            optimization: HasherOptimization::default(),
            walk_strategy: HasherWalkStrategy::default(),
            warn_on_missing_inputs: true,
        }
    }
}
