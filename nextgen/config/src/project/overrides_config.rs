use moon_common::cacheable;
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::Config;
use version_spec::UnresolvedVersionSpec;

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
        /// Disable all TypeScript functionality for this project.
        pub disabled: bool,
        /// Append project reference sources to `include` in `tsconfig.json`.
        pub include_project_reference_sources: Option<bool>,
        /// Append shared types to `include` in `tsconfig.json`.
        pub include_shared_types: Option<bool>,
        /// Update and route `outDir` in `tsconfig.json` to moon's cache.
        pub route_out_dir_to_cache: Option<bool>,
        /// Sync all project dependencies as `references` in `tsconfig.json`.
        pub sync_project_references: Option<bool>,
        /// Sync all project dependencies as `paths` in `tsconfig.json`.
        pub sync_project_references_to_paths: Option<bool>,
    }
);

cacheable!(
    /// Overrides top-level toolchain settings, scoped to this project.
    #[derive(Clone, Config, Debug, PartialEq)]
    pub struct ProjectToolchainConfig {
        /// Overrides `bun` settings.
        #[setting(nested)]
        pub bun: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `deno` settings.
        #[setting(nested)]
        pub deno: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `node` settings.
        #[setting(nested)]
        pub node: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `rust` settings.
        #[setting(nested)]
        pub rust: Option<ProjectToolchainCommonToolConfig>,

        /// Overrides `typescript` settings.
        #[setting(nested)]
        pub typescript: Option<ProjectToolchainTypeScriptConfig>,
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
