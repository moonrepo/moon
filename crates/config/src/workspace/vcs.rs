use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VcsManager {
    Git,
    Svn,
}

impl Default for VcsManager {
    fn default() -> Self {
        VcsManager::Git
    }
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
