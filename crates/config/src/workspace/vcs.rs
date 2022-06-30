use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VcsManager {
    #[default]
    Git,
    Svn,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct VcsConfig {
    pub manager: VcsManager,

    pub default_branch: String,
}

impl Default for VcsConfig {
    fn default() -> Self {
        VcsConfig {
            manager: VcsManager::default(),
            default_branch: String::from("master"),
        }
    }
}
