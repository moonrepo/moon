use crate::language_platform::{LanguageType, PlatformType};
use crate::project::*;
use crate::shapes::InputPath;
use crate::{config_enum, config_struct, config_unit_enum};
use moon_common::Id;
use rustc_hash::FxHashMap;
use schematic::{Config, ConfigEnum, ValidateError, validate};
use std::collections::BTreeMap;

fn validate_channel<D, C>(
    value: &str,
    _data: &D,
    _ctx: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    if !value.is_empty() && !value.starts_with('#') {
        return Err(ValidateError::new("must start with a `#`"));
    }

    Ok(())
}

config_unit_enum!(
    /// The technology stack of the project, for categorizing.
    #[derive(ConfigEnum)]
    pub enum StackType {
        Backend,
        Frontend,
        Infrastructure,
        Systems,
        #[default]
        Unknown,
    }
);

config_unit_enum!(
    /// The layer within the project stack, for categorizing.
    #[derive(ConfigEnum)]
    pub enum LayerType {
        Application,
        Automation,
        Configuration,
        Library,
        Scaffolding,
        Tool,
        #[default]
        Unknown,
    }
);

config_struct!(
    /// Expanded information about the project.
    #[derive(Config)]
    pub struct ProjectMetadataConfig {
        /// A human-readable name of the project.
        pub name: Option<String>,

        /// A description on what the project does, and why it exists.
        #[setting(validate = validate::not_empty)]
        pub description: String,

        /// The owner of the project. Can be an individual, team, or
        /// organization. The format is unspecified.
        pub owner: Option<String>,

        /// The individual maintainers of the project. The format is unspecified.
        pub maintainers: Vec<String>,

        /// The Slack, Discord, etc, channel to discuss the project.
        /// Must start with a `#`.
        #[setting(validate = validate_channel)]
        pub channel: Option<String>,

        /// Custom metadata fields.
        pub metadata: FxHashMap<String, serde_json::Value>,
    }
);

config_enum!(
    /// Expanded information about a project dependency.
    #[derive(Config)]
    #[serde(
        untagged,
        expecting = "expected a project name or dependency config object"
    )]
    pub enum ProjectDependsOn {
        /// A project referenced by ID.
        String(Id),

        /// A project referenced by ID, with additional parameters to pass through.
        #[setting(nested)]
        Object(DependencyConfig),
    }
);

config_struct!(
    /// Configures information and tasks for a project.
    /// Docs: https://moonrepo.dev/docs/config/project
    #[derive(Config)]
    pub struct ProjectConfig {
        #[setting(
            default = "https://moonrepo.dev/schemas/project.json",
            rename = "$schema"
        )]
        pub schema: String,

        /// Other projects that this project depends on.
        #[setting(nested)]
        pub depends_on: Vec<ProjectDependsOn>,

        /// Configures Docker integration for this project.
        #[setting(nested)]
        pub docker: ProjectDockerConfig,

        /// A mapping of environment variables that will be set for
        /// all tasks within the project.
        pub env: FxHashMap<String, String>,

        /// A mapping of group IDs to a list of file paths, globs, and
        /// environment variables, that can be referenced from tasks.
        pub file_groups: FxHashMap<Id, Vec<InputPath>>,

        /// Overrides the ID within the project graph, as defined in
        /// the workspace `projects` setting.
        pub id: Option<Id>,

        /// The primary programming language of the project.
        pub language: LanguageType,

        /// The layer within the project stack, for categorizing.
        #[serde(alias = "type")]
        pub layer: LayerType,

        /// Defines ownership of source code within the current project, by mapping
        /// file paths and globs to owners. An owner is either a user, team, or group.
        #[setting(nested)]
        pub owners: OwnersConfig,

        /// The default platform for all tasks within the project,
        /// if their platform is unknown.
        #[deprecated]
        pub platform: Option<PlatformType>,

        /// Expanded information about the project.
        #[setting(nested)]
        pub project: Option<ProjectMetadataConfig>,

        /// The technology stack of the project, for categorizing.
        pub stack: StackType,

        /// A list of tags that this project belongs to, for categorizing,
        /// boundary enforcement, and task inheritance.
        pub tags: Vec<Id>,

        /// A mapping of tasks by ID to parameters required for running the task.
        #[setting(nested)]
        pub tasks: BTreeMap<Id, TaskConfig>,

        /// Overrides top-level toolchain settings, scoped to this project.
        #[setting(nested)]
        pub toolchain: ProjectToolchainConfig,

        /// Overrides top-level workspace settings, scoped to this project.
        #[setting(nested)]
        pub workspace: ProjectWorkspaceConfig,
    }
);
