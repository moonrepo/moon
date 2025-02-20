use crate::toolchain::ToolchainPluginConfig;
use moon_common::cacheable;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;

cacheable!(
    #[derive(Clone, Config, Debug, PartialEq)]
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
}

cacheable!(
    /// Overrides top-level toolchain settings.
    #[derive(Clone, Config, Debug, PartialEq)]
    pub struct ProjectToolchainCommonToolConfig {
        /// Version of the tool this project will use.
        pub version: Option<UnresolvedVersionSpec>,
    }
);

cacheable!(
    /// Overrides top-level `typescript` settings.
    #[derive(Clone, Config, Debug, PartialEq)]
    pub struct ProjectToolchainTypeScriptConfig {
        /// Disables all TypeScript functionality for this project.
        pub disabled: bool,

        /// Appends sources of project reference to `include` in `tsconfig.json`.
        pub include_project_reference_sources: Option<bool>,

        /// Appends shared types to `include` in `tsconfig.json`.
        pub include_shared_types: Option<bool>,

        /// Updates and routes `outDir` in `tsconfig.json` to moon's cache.
        pub route_out_dir_to_cache: Option<bool>,

        /// Syncs all project dependencies as `references` in `tsconfig.json`.
        pub sync_project_references: Option<bool>,

        /// Syncs all project dependencies as `paths` in `tsconfig.json`.
        pub sync_project_references_to_paths: Option<bool>,
    }
);

cacheable!(
    /// Overrides top-level toolchain settings, scoped to this project.
    #[derive(Clone, Config, Debug, PartialEq)]
    #[config(allow_unknown_fields)]
    pub struct ProjectToolchainConfig {
        /// The default toolchain for all tasks within the project,
        /// if their toolchain is unknown.
        pub default: Option<Id>,

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

        /// Overrides `typescript` settings.
        #[setting(nested)]
        pub typescript: Option<ProjectToolchainTypeScriptConfig>,

        /// Overrides toolchains by their ID.
        #[setting(flatten, nested)]
        pub toolchains: FxHashMap<Id, ProjectToolchainEntry>,
    }
);

impl ProjectToolchainConfig {
    pub fn is_typescript_enabled(&self) -> bool {
        self.typescript
            .as_ref()
            .map(|ts| !ts.disabled)
            .unwrap_or(true)
    }
}

cacheable!(
    /// Controls how tasks are inherited.
    #[derive(Clone, Config, Debug, PartialEq)]
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

cacheable!(
    /// Overrides top-level workspace settings, scoped to this project.
    #[derive(Clone, Config, Debug, PartialEq)]
    pub struct ProjectWorkspaceConfig {
        /// Controls how tasks are inherited.
        #[setting(nested)]
        pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
    }
);
