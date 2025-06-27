use crate::config_struct;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;

config_struct!(
    /// Configures boundaries and constraints between projects.
    #[derive(Config)]
    pub struct ConstraintsConfig {
        /// Enforces relationships between projects based on each project's
        /// `layer` setting.
        #[setting(default = true, alias = "enforceProjectTypeRelationships")]
        pub enforce_layer_relationships: bool,

        /// Enforces relationships between projects based on each project's
        /// `tags` setting. Requires a mapping of tags, to acceptable tags.
        pub tag_relationships: FxHashMap<Id, Vec<Id>>,
    }
);
