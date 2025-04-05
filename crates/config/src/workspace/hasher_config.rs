use crate::portable_path::GlobPath;
use crate::{config_struct, config_unit_enum};
use schematic::{Config, ConfigEnum};

config_unit_enum!(
    /// The optimization to use when hashing.
    #[derive(ConfigEnum)]
    pub enum HasherOptimization {
        /// Prefer accuracy, but slower hashing.
        #[default]
        Accuracy,
        /// Prefer performance, but less accurate hashing.
        Performance,
    }
);

config_unit_enum!(
    /// The strategy to use when walking the file system.
    #[derive(ConfigEnum)]
    pub enum HasherWalkStrategy {
        /// Glob the file system.
        Glob,
        /// Query the VCS.
        #[default]
        Vcs,
    }
);

config_struct!(
    /// Configures aspects of the content hashing engine.
    #[derive(Config)]
    pub struct HasherConfig {
        /// The number of files to include in each hash operation.
        #[deprecated]
        #[setting(default = 2500)]
        pub batch_size: u16,

        /// Filters file paths that match a configured glob pattern
        /// when a hash is being generated. Patterns are workspace relative,
        /// so prefixing with `**` is recommended.
        pub ignore_patterns: Vec<GlobPath>,

        /// When `warnOnMissingInputs` is enabled, filters missing file
        /// paths from logging a warning.
        pub ignore_missing_patterns: Vec<GlobPath>,

        /// The optimization to use when hashing.
        pub optimization: HasherOptimization,

        /// The strategy to use when walking the file system.
        pub walk_strategy: HasherWalkStrategy,

        /// Logs a warning when a task has configured an explicit file path
        /// input, and that file does not exist when hashing.
        #[setting(default = true)]
        pub warn_on_missing_inputs: bool,
    }
);
