use crate::config_struct;
use crate::shapes::GlobPath;
use crate::workspace::{DockerFileConfig, PartialDockerFileConfig};
use schematic::Config;

config_struct!(
    /// Configures aspects of the Docker scaffolding process.
    /// @since 1.27.0
    #[derive(Config)]
    pub struct ProjectDockerScaffoldConfig {
        /// A list of glob patterns, relative from the project root,
        /// to include (or exclude) in the "sources" skeleton.
        pub include: Vec<GlobPath>,
    }
);

config_struct!(
    /// Configures our Docker integration.
    /// @since 1.27.0
    #[derive(Config)]
    pub struct ProjectDockerConfig {
        /// Configures aspects of the `Dockerfile` generation process.
        /// @since 1.27.0
        #[setting(nested)]
        pub file: DockerFileConfig,

        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: ProjectDockerScaffoldConfig,
    }
);
