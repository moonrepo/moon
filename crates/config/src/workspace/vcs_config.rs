use crate::{config_struct, config_unit_enum};
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigEnum};

config_unit_enum!(
    /// The VCS being utilized by the repository.
    #[derive(ConfigEnum)]
    pub enum VcsClient {
        #[default]
        Git,
    }
);

config_unit_enum!(
    /// The upstream version control provider, where the repository
    /// source code is stored.
    #[derive(ConfigEnum)]
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

config_unit_enum!(
    /// The format to use for generated VCS hook files.
    /// @since 1.29.0
    #[derive(ConfigEnum)]
    pub enum VcsHookFormat {
        Bash,
        #[default]
        Native,
    }
);

config_struct!(
    /// Configures the version control system (VCS).
    #[derive(Config)]
    pub struct VcsConfig {
        /// The VCS client being utilized by the repository.
        pub client: VcsClient,

        /// The default branch / base.
        #[setting(default = "master")]
        pub default_branch: String,

        /// A map of hooks to a list of commands to run when the hook is triggered.
        /// @since 1.9.0
        pub hooks: FxHashMap<String, Vec<String>>,

        /// The format to use for generated VCS hook files.
        /// @since 1.29.0
        pub hook_format: VcsHookFormat,

        /// The upstream version control provider, where the repository
        /// source code is stored.
        /// @since 1.8.0
        pub provider: VcsProvider,

        /// List of remote's in which to compare branches against.
        #[setting(default = vec!["origin".into(), "upstream".into()])]
        pub remote_candidates: Vec<String>,

        /// Automatically generate hooks and scripts during a sync operation,
        /// based on the `hooks` setting.
        /// @since 1.9.0
        pub sync: bool,
    }
);
