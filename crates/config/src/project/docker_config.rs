use crate::config_struct;
use crate::workspace::{
    DockerFileConfig, DockerScaffoldConfig, PartialDockerFileConfig, PartialDockerScaffoldConfig,
};
use schematic::Config;

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
        /// @since 1.27.0
        #[setting(nested)]
        pub scaffold: DockerScaffoldConfig,
    }
);
