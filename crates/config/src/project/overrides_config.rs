use crate::shapes::OneOrMany;
use crate::toolchain::ToolchainPluginConfig;
use crate::{config_enum, config_struct};
use moon_common::Id;
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
            Self::Config(config) => !config.disabled,
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
        /// The default toolchain(s) for all tasks within the project,
        /// if their toolchain is unknown.
        pub default: Option<OneOrMany<Id>>,

        /// Overrides `bun` settings.
        #[setting(nested)]
        pub bun: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `deno` settings.
        #[setting(nested)]
        pub deno: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `python` settings.
        #[setting(nested)]
        pub python: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `node` settings.
        #[setting(nested)]
        pub node: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `rust` settings.
        #[setting(nested)]
        pub rust: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides toolchains by their ID.
        #[setting(flatten, nested)]
        pub plugins: FxHashMap<Id, ProjectToolchainEntry>,
    }
);

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
