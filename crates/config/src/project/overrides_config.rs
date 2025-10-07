use crate::patterns::merge_plugin_partials;
use crate::shapes::OneOrMany;
use crate::toolchain_config::ToolchainPluginConfig;
use crate::{config_enum, config_struct};
use moon_common::{Id, IdExt};
use rustc_hash::FxHashMap;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;

config_enum!(
    #[derive(Config)]
    #[serde(untagged)]
    pub enum ProjectToolchainEntry {
        Disabled, // null
        Enabled(bool),
        #[setting(nested)]
        Config(ToolchainPluginConfig),
    }
);

impl ProjectToolchainEntry {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Disabled => false,
            Self::Enabled(state) => *state,
            Self::Config(_) => true,
        }
    }

    pub fn get_version(&self) -> Option<&UnresolvedVersionSpec> {
        match self {
            Self::Config(config) => config.version.as_ref(),
            _ => None,
        }
    }
}

config_struct!(
    /// Overrides top-level toolchain settings.
    #[derive(Config)]
    pub struct ProjectToolchainCommonToolConfig {
        /// Version of the tool this project will use.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

config_struct!(
    /// Overrides top-level toolchain settings, scoped to this project.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ProjectToolchainConfig {
        /// The default toolchain(s) to inherit for the project,
        /// and all of its tasks.
        #[serde(alias = "defaults")]
        pub default: Option<OneOrMany<Id>>,

        /// Overrides toolchains by their ID.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ProjectToolchainEntry>,
    }
);

impl ProjectToolchainConfig {
    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ProjectToolchainEntry> {
        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        self.plugins
            .get(&stable_id)
            .or_else(|| self.plugins.get(&unstable_id))
    }
}

config_struct!(
    /// Controls how tasks are inherited.
    #[derive(Config)]
    pub struct ProjectWorkspaceInheritedTasksConfig {
        /// Excludes inheriting tasks by ID.
        pub exclude: Vec<Id>,

        /// Only inherits tasks by ID, and ignores the rest.
        /// When not defined, inherits all matching tasks.
        /// When an empty list, inherits no tasks.
        pub include: Option<Vec<Id>>,

        /// Renames inherited tasks to a new ID.
        pub rename: FxHashMap<Id, Id>,
    }
);

config_struct!(
    /// Overrides top-level workspace settings, scoped to this project.
    #[derive(Config)]
    pub struct ProjectWorkspaceConfig {
        /// Controls how tasks are inherited.
        #[setting(nested)]
        pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
    }
);
