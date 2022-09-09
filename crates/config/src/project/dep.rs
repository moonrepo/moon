use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyScope {
    Development,
    Peer,
    #[default]
    Production,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
pub struct DependencyConfig {
    pub id: String,
    pub scope: DependencyScope,

    // This field isn't configured by users, but is used by platforms!
    #[schemars(skip)]
    pub via: Option<String>,
}

impl DependencyConfig {
    pub fn new(id: &str) -> Self {
        DependencyConfig {
            id: id.to_owned(),
            scope: DependencyScope::Production,
            via: None,
        }
    }
}
