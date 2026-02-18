use crate::{config_struct, config_unit_enum, is_false};
use indexmap::IndexMap;
use schematic::{Config, ConfigEnum};

config_unit_enum!(
    /// How to order ownership rules within the generated file.
    #[derive(ConfigEnum)]
    pub enum CodeownersOrderBy {
        /// By file source path.
        #[default]
        FileSource,
        /// By project identifier.
        ProjectId,
    }
);

config_struct!(
    /// Configures code ownership rules for generating a `CODEOWNERS` file.
    #[derive(Config)]
    pub struct CodeownersConfig {
        /// A map of global file paths and glob patterns to a list of owners.
        /// Can be relative from the workspace root, or a wildcard match for any depth.
        #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
        pub global_paths: IndexMap<String, Vec<String>>,

        /// How to order ownership rules within the generated file.
        pub order_by: CodeownersOrderBy,

        /// Bitbucket and GitLab only. The number of approvals required for the
        /// request to be satisfied. This will be applied to all paths.
        /// @since 1.28.0
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub required_approvals: Option<u8>,

        /// Automatically generate a `CODEOWNERS` file during a sync operation,
        /// after aggregating all ownership rules from each project in the workspace.
        #[serde(default, skip_serializing_if = "is_false")]
        pub sync: bool,
    }
);
