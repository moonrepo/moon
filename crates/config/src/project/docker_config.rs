use crate::config_struct;
use crate::portable_path::GlobPath;
use moon_common::Id;
use schematic::Config;

config_struct!(
    /// Configures `Dockerfile` generation.
    #[derive(Config)]
    pub struct ProjectDockerFileConfig {
        /// A task within the current project for building the project.
        pub build_task: Option<Id>,

        /// The base Docker image.
        pub image: Option<String>,

        /// A task within the current project for starting the project.
        pub start_task: Option<Id>,
    }
);

config_struct!(
    /// Configures aspects of the Docker scaffolding process.
    #[derive(Config)]
    pub struct ProjectDockerScaffoldConfig {
        /// List of glob patterns, relative from the project root,
        /// to include (or exclude) in the sources skeleton.
        pub include: Vec<GlobPath>,
    }
);

config_struct!(
    /// Configures our Docker integration.
    #[derive(Config)]
    pub struct ProjectDockerConfig {
        /// Configures aspects of the `Dockerfile` generation process.
        #[setting(nested)]
        pub file: ProjectDockerFileConfig,

        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: ProjectDockerScaffoldConfig,
    }
);
