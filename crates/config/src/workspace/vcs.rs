use serde::{Deserialize, Serialize};
use validator::Validate;

fn default_branch_default() -> String {
    String::from("origin/master")
}

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
#[serde(rename_all = "camelCase")]
pub struct VcsConfig {
    #[serde(default)]
    pub manager: VcsManager,

    #[serde(default = "default_branch_default")]
    pub default_branch: String,
}

impl Default for VcsConfig {
    fn default() -> Self {
        VcsConfig {
            manager: VcsManager::default(),
            default_branch: default_branch_default(),
        }
    }
}
