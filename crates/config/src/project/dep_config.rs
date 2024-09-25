use moon_common::{cacheable, Id};
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
    /// The scope and or relationship of the dependency.
    #[derive(ConfigEnum, Copy, Default)]
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

derive_enum!(
    /// The source where the dependency comes from. Either explicitly
    /// defined in configuration, or implicitly derived from source files.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum DependencySource {
        #[default]
        Explicit,
        Implicit,
    }
);

cacheable!(
    /// Expanded information about a project dependency.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
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
