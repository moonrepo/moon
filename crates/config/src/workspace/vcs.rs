use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct VcsConfig {
    pub manager: Option<VcsManager>,

    #[serde(rename = "defaultBranch")]
    pub default_branch: Option<String>,
}

impl Default for VcsConfig {
    fn default() -> Self {
        VcsConfig {
            manager: Some(VcsManager::default()),
            default_branch: Some(String::from("origin/master")),
        }
    }
}
