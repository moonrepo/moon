use schematic::{config_enum, Config};
use strum::Display;

config_enum!(
    #[derive(Default, Display)]
    pub enum VcsManager {
        #[strum(serialize = "git")]
        #[default]
        Git,

        #[strum(serialize = "svn")]
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
