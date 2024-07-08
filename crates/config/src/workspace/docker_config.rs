use crate::portable_path::GlobPath;
use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures aspects of the Docker scaffolding process.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct DockerScaffoldConfig {
        /// Copy toolchain specific configs/manifests/files into
        /// the workspace skeleton.
        #[setting(default = true)]
        pub copy_toolchain_files: bool,

        /// List of glob patterns, relative from the workspace root,
        /// to include (or exclude) in the workspace skeleton.
        pub include: Vec<GlobPath>,
    }
);

cacheable!(
    /// Configures our Docker integration.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct DockerConfig {
        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: DockerScaffoldConfig,
    }
);
