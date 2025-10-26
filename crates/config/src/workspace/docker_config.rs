use crate::config_struct;
use crate::shapes::{FilePath, GlobPath};
use moon_common::Id;
use schematic::Config;

config_struct!(
    /// Configures `Dockerfile` generation. When configured at the workspace-level,
    /// applies to all projects, but can be overridden at the project-level.
    #[derive(Config)]
    pub struct DockerFileConfig {
        /// A task identifier within the current project for building the project.
        /// If not defined, will skip the build step.
        pub build_task: Option<Id>,

        /// The base Docker image to use. If not defined, will use the provided image
        /// from the first matching toolchain, otherwise defaults to "scratch".
        pub image: Option<String>,

        /// Run the `moon docker prune` command after building the
        /// project, but before starting it.
        /// @since 2.0.0
        pub run_prune: Option<bool>,

        /// Run the `moon docker setup` command after scaffolding,
        /// but before building the project.
        /// @since 2.0.0
        pub run_setup: Option<bool>,

        /// A task identifier within the current project for starting the project
        /// within the `CMD` instruction. If not defined, will skip the start step
        /// and not include the `CMD` instruction.
        pub start_task: Option<Id>,

        /// A template file, relative from the workspace root, to use when rendering
        /// the `Dockerfile`. Powered by Tera.
        pub template: Option<FilePath>,
    }
);

config_struct!(
    /// Configures aspects of the Docker pruning process.
    #[derive(Config)]
    pub struct DockerPruneConfig {
        /// Automatically delete vendor directories (package manager
        /// dependencies, build targets, etc) while pruning. This is
        /// handled by each toolchain plugin and not moon directly.
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
    /// When configured at the workspace-level, applies to all projects,
    /// but can be overridden at the project-level.
    #[derive(Config)]
    pub struct DockerScaffoldConfig {
        /// List of glob patterns in which to include/exclude files in
        /// the "configs" skeleton. Applies to both project and
        /// workspace level scaffolding.
        pub configs_phase_globs: Vec<GlobPath>,

        /// List of glob patterns in which to include/exclude files in
        /// the "sources" skeleton. Applies to both project and
        /// workspace level scaffolding.
        pub sources_phase_globs: Vec<GlobPath>,
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
