use crate::portable_path::GlobPath;
use moon_common::cacheable;
use schematic::Config;

cacheable!(
    /// Configures aspects of the Docker scaffolding process.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct ProjectDockerScaffoldConfig {
        /// List of glob patterns, relative from the project root,
        /// to include (or exclude) in the sources skeleton.
        pub include: Vec<GlobPath>,
    }
);

cacheable!(
    /// Configures our Docker integration.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct ProjectDockerConfig {
        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: ProjectDockerScaffoldConfig,
    }
);
