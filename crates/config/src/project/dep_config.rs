use crate::{config_struct, config_unit_enum};
use moon_common::Id;
use schematic::{Config, ConfigEnum};

config_unit_enum!(
    /// The task-to-task relationship of the dependency.
    #[derive(ConfigEnum)]
    pub enum DependencyType {
        Cleanup,
        #[default]
        Required,
        Optional,
    }
);

config_unit_enum!(
    /// The scope and or relationship of the dependency.
    #[derive(ConfigEnum)]
    pub enum DependencyScope {
        Build,
        Development,
        Peer,
        #[default]
        Production,

        // Special case when depending on the root-level project
        Root,
    }
);

config_unit_enum!(
    /// The source where the dependency comes from. Either explicitly
    /// defined in configuration, or implicitly derived from source files.
    #[derive(ConfigEnum)]
    pub enum DependencySource {
        #[default]
        Explicit,
        Implicit,
    }
);

config_struct!(
    /// Expanded information about a project dependency.
    #[derive(Config)]
    pub struct DependencyConfig {
        /// ID of the depended on project.
        pub id: Id,

        /// Scope of the dependency relationship.
        pub scope: DependencyScope,

        /// Source of where the dependency came from.
        pub source: DependencySource,

        /// Metadata about the source.
        pub via: Option<String>,
    }
);

impl DependencyConfig {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn is_build_scope(&self) -> bool {
        matches!(self.scope, DependencyScope::Build)
    }

    pub fn is_root_scope(&self) -> bool {
        matches!(self.scope, DependencyScope::Root)
    }
}
