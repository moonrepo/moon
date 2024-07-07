use crate::portable_path::GlobPath;
use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures aspects of the Docker scaffolding process.
    /// When configured in a project, paths are relative from the project root.
    /// When configured at the workspace, paths are relative from the workspace root.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct DockerScaffoldConfig {
        /// Copy toolchain specific configs/manifests/files into the workspace skeleton.
        #[setting(default = true)]
        pub copy_toolchain_files: bool,

        /// List of glob patterns to include (or exclude) in the sources skeleton.
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
