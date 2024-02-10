use moon_common::cacheable;
use rustc_hash::FxHashMap;
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    /// How to order ownership rules within the generated file.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum CodeownersOrderBy {
        /// By file source path.
        #[default]
        FileSource,
        /// By project name.
        ProjectName,
    }
);

cacheable!(
    /// Configures code ownership rules for generating a `CODEOWNERS` file.
    #[derive(Clone, Config, Debug)]
    pub struct CodeownersConfig {
        /// Paths that are applied globally to all projects. Can be relative
        /// from the workspace root, or a wildcard match for any depth.
        pub global_paths: FxHashMap<String, Vec<String>>,

        /// How to order ownership rules within the generated file.
        pub order_by: CodeownersOrderBy,

        /// Generates a `CODEOWNERS` file after aggregating all ownership
        /// rules from each project in the workspace.
        pub sync_on_run: bool,
    }
);
