use crate::patterns::merge_plugin_partials;
use crate::shapes::OneOrMany;
use crate::toolchains_config::ToolchainPluginConfig;
use crate::{config_enum, config_struct};
use moon_common::{Id, IdExt};
use rustc_hash::FxHashMap;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;

config_enum!(
    /// Variants a project-level toolchain can be configured.
    #[derive(Config)]
    #[serde(untagged)]
    pub enum ProjectToolchainEntry {
        Disabled, // null
        Enabled(bool),
        #[setting(nested)]
        Object(ToolchainPluginConfig),
    }
);

impl ProjectToolchainEntry {
    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Disabled => false,
            Self::Enabled(state) => *state,
            Self::Object(_) => true,
        }
    }

    pub fn get_version(&self) -> Option<&UnresolvedVersionSpec> {
        match self {
            Self::Object(config) => config.version.as_ref(),
            _ => None,
        }
    }
}

config_struct!(
    /// Overrides top-level toolchain settings, scoped to this project.
    #[derive(Config)]
    #[config(allow_unknown_fields)]
    pub struct ProjectToolchainsConfig {
        /// A single toolchain, or list of toolchains, to inherit for
        /// this project and all of its tasks.
        /// @since 1.31.0
        #[setting(alias = "defaults")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub default: Option<OneOrMany<Id>>,

        /// Overrides workspace-level toolchains by their identifier.
        #[setting(flatten, nested, merge = merge_plugin_partials)]
        pub plugins: FxHashMap<Id, ProjectToolchainEntry>,
    }
);

impl ProjectToolchainsConfig {
    pub fn get_plugin_config(&self, id: impl AsRef<str>) -> Option<&ProjectToolchainEntry> {
        let (stable_id, unstable_id) = Id::stable_and_unstable(id);

        self.plugins
            .get(&stable_id)
            .or_else(|| self.plugins.get(&unstable_id))
    }
}

config_struct!(
    /// Controls how workspace-level tasks are inherited.
    #[derive(Config)]
    pub struct ProjectWorkspaceInheritedTasksConfig {
        /// Excludes inheriting tasks by their identifier.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub exclude: Vec<Id>,

        /// Only inherits tasks with the provided identifiers,
        /// and ignores the rest. When not defined, inherits
        /// all matching tasks. When an empty list, inherits no tasks.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub include: Option<Vec<Id>>,

        /// Renames inherited tasks by mapping their existing
        /// identifier to a new identifier, scoped to this project.
        #[serde(default, skip_serializing_if = "FxHashMap::is_empty")]
        pub rename: FxHashMap<Id, Id>,
    }
);

config_struct!(
    /// Overrides workspace settings, scoped to this project.
    #[derive(Config)]
    pub struct ProjectWorkspaceConfig {
        /// Controls how tasks are inherited.
        #[setting(nested)]
        pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
    }
);
