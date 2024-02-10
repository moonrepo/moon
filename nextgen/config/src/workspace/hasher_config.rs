use crate::portable_path::GlobPath;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    /// The optimization to use when hashing.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum HasherOptimization {
        /// Prefer accuracy, but slower hashing.
        #[default]
        Accuracy,
        /// Prefer performance, but less accurate hashing.
        Performance,
    }
);

derive_enum!(
    /// The strategy to use when walking the file system.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum HasherWalkStrategy {
        /// Glob the file system.
        Glob,
        /// Query the VCS.
        #[default]
        Vcs,
    }
);

/// Configures aspects of the content hashing engine.
#[derive(Clone, Config, Debug)]
pub struct HasherConfig {
    /// The number of files to include in each hash operation.
    #[setting(default = 2500)]
    pub batch_size: u16,

    /// Filters file paths that match a configured glob pattern
    /// when a hash is being generated. Patterns are workspace relative,
    /// so prefixing with `**/*` is recommended.
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
