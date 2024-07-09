use crate::portable_path::GlobPath;
use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures aspects of the Docker pruning process.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct DockerPruneConfig {
        /// Automatically delete vendor directories (package manager
        /// dependencies, build targets, etc) while pruning.
        #[setting(default = true)]
        pub delete_vendor_directories: bool,

        /// Automatically install production dependencies for all required
        /// toolchain's of the focused projects within the Docker build.
        #[setting(default = true)]
        pub install_toolchain_deps: bool,
    }
);

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
        /// Configures aspects of the Docker pruning process.
        #[setting(nested)]
        pub prune: DockerPruneConfig,

        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: DockerScaffoldConfig,
    }
);
