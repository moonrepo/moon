use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RepoType {
    #[default]
    Unknown,
    Monorepo,
    MonorepoWithRoot,
    Polyrepo,
}

impl RepoType {
    pub fn is_monorepo(&self) -> bool {
        matches!(self, Self::Monorepo | Self::MonorepoWithRoot)
    }
}
