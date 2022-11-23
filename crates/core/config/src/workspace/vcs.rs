use moon_utils::string_vec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::Display;
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VcsManager {
    #[strum(serialize = "git")]
    #[default]
    Git,

    #[strum(serialize = "svn")]
    Svn,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct VcsConfig {
    pub default_branch: String,

    pub manager: VcsManager,

    pub remote_candidates: Vec<String>,
}

impl Default for VcsConfig {
    fn default() -> Self {
        VcsConfig {
            default_branch: "master".into(),
            manager: VcsManager::default(),
            remote_candidates: string_vec!["origin", "upstream"],
        }
    }
}
