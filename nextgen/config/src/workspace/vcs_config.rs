use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum VcsManager {
        #[default]
        Git,
        // Svn,
    }
);

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum VcsProvider {
        Bitbucket,

        #[default]
        #[serde(rename = "github")]
        GitHub,

        #[serde(rename = "gitlab")]
        GitLab,

        Other,
    }
);

#[derive(Clone, Config)]
pub struct VcsConfig {
    #[setting(default = "master")]
    pub default_branch: String,

    pub hooks: FxHashMap<String, Vec<String>>,

    pub manager: VcsManager,

    pub provider: VcsProvider,

    #[setting(default = vec!["origin".into(), "upstream".into()])]
    pub remote_candidates: Vec<String>,

    pub sync_hooks_on_run: bool,
}
