use moon_common::{cacheable, Id};
use schematic::{derive_enum, Config, ConfigEnum};

derive_enum!(
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
    #[derive(ConfigEnum, Copy, Default)]
    pub enum DependencySource {
        #[default]
        Explicit,
        Implicit,
    }
);

cacheable!(
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct DependencyConfig {
        pub id: Id,
        pub scope: DependencyScope,
        pub source: DependencySource,
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
}
