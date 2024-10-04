use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    /// The VCS being utilized by the repository.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum VcsManager {
        #[default]
        Git,
    }
);

derive_enum!(
    /// The upstream version control provider, where the repository
    /// source code is stored.
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

/// Configures the version control system (VCS).
#[derive(Clone, Config, Debug, PartialEq)]
pub struct VcsConfig {
    /// The default branch / base.
    #[setting(default = "master")]
    pub default_branch: String,

    /// A mapping of hooks to commands to run when the hook is triggered.
    pub hooks: FxHashMap<String, Vec<String>>,

    /// The VCS client being utilized by the repository.
    pub manager: VcsManager,

    /// The upstream version control provider, where the repository
    /// source code is stored.
    pub provider: VcsProvider,

    /// List of remote's in which to compare branches against.
    #[setting(default = vec!["origin".into(), "upstream".into()])]
    pub remote_candidates: Vec<String>,

    /// Generates hooks and scripts based on the `hooks` setting.
    pub sync_hooks: bool,
}
