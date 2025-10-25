use crate::config_struct;
use crate::shapes::GlobPath;
use moon_common::Id;
use schematic::Config;

config_struct!(
    /// Configures `Dockerfile` generation.
    #[derive(Config)]
    pub struct DockerFileConfig {
        /// A task identifier within the current project for building the project.
        pub build_task: Option<Id>,

        /// The base Docker image to use.
        pub image: Option<String>,

        /// Run the `moon docker prune` command after building the
        /// project, but before starting it.
        /// @since 2.0.0
        pub run_prune: Option<bool>,

        /// Run the `moon docker setup` command after scaffolding,
        /// but before building the project.
        /// @since 2.0.0
        pub run_setup: Option<bool>,

        /// A task identifier within the current project for starting the project.
        pub start_task: Option<Id>,
    }
);

config_struct!(
    /// Configures aspects of the Docker pruning process.
    #[derive(Config)]
    pub struct DockerPruneConfig {
        /// Automatically delete vendor directories (package manager
        /// dependencies, build targets, etc) while pruning. This is
        /// handled by each toolchain plugin.
        #[setting(default = true)]
        pub delete_vendor_directories: bool,

        /// Automatically install production dependencies for all required
        /// toolchain's of the focused projects within the Docker build.
        #[setting(default = true)]
        pub install_toolchain_dependencies: bool,
    }
);

config_struct!(
    /// Configures aspects of the Docker scaffolding process.
    #[derive(Config)]
    pub struct DockerScaffoldConfig {
        /// List of glob patterns, relative from the workspace root,
        /// to include (or exclude) in the "configs" skeleton.
        pub include: Vec<GlobPath>,
    }
);

config_struct!(
    /// Configures our Docker integration.
    #[derive(Config)]
    pub struct DockerConfig {
        /// Configures aspects of the `Dockerfile` generation process.
        /// @since 2.0.0
        #[setting(nested)]
        pub file: DockerFileConfig,

        /// Configures aspects of the Docker pruning process.
        #[setting(nested)]
        pub prune: DockerPruneConfig,

        /// Configures aspects of the Docker scaffolding process.
        #[setting(nested)]
        pub scaffold: DockerScaffoldConfig,
    }
);
