use schematic::{derive_enum, Config, ConfigEnum};
use serde::Serialize;

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum VcsManager {
        #[default]
        Git,
        Svn,
    }
);

#[derive(Clone, Config)]
pub struct VcsConfig {
    #[setting(default = "master")]
    pub default_branch: String,

    pub manager: VcsManager,

    #[setting(default = vec!["origin".into(), "upstream".into()])]
    pub remote_candidates: Vec<String>,
}
