use moon_common::Id;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::Display;
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyScope {
    #[strum(serialize = "development")]
    Development,

    #[strum(serialize = "peer")]
    Peer,

    #[default]
    #[strum(serialize = "production")]
    Production,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default, deny_unknown_fields)]
pub struct DependencyConfig {
    pub id: Id,
    pub scope: DependencyScope,
    #[schemars(skip)]
    pub via: Option<String>,
}

impl DependencyConfig {
    pub fn new(id: &str) -> Self {
        DependencyConfig {
            id: Id::raw(id),
            scope: DependencyScope::Production,
            via: None,
        }
    }
}
