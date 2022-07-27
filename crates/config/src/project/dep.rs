use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyScope {
    Development,
    Peer,
    #[default]
    Production,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
pub struct DependencyConfig {
    pub id: String,

    pub scope: DependencyScope,
}

impl DependencyConfig {
    pub fn new(id: &str) -> Self {
        DependencyConfig {
            id: id.to_owned(),
            scope: DependencyScope::Production,
        }
    }
}
