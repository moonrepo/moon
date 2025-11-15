use crate::context::{MoonContext, Operation};
use crate::is_false;
use crate::prompts::SettingPrompt;
use moon_common::Id;
use moon_config::{DependencyScope, PartialTaskConfig};
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use rustc_hash::FxHashMap;
use schematic::Schema;
use std::collections::BTreeMap;
use std::path::PathBuf;
use warpgate_api::*;

pub type ConfigSchema = Schema;

// INIT

api_struct!(
    /// Input passed to the initialize functions.
    pub struct InitializePluginInput {
        /// Current moon context.
        pub context: MoonContext,
    }
);

api_struct!(
    /// Output returned from the initialize functions.
    #[serde(default)]
    pub struct InitializePluginOutput {
        /// A URL to documentation about available configuration settings.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub config_url: Option<String>,

        /// Settings to include in the injected toolchain config file.
        /// Supports dot notation for the keys.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub default_settings: FxHashMap<String, serde_json::Value>,

        /// A URL to documentation about the toolchain.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub docs_url: Option<String>,

        /// A list of questions to prompt the user about configuration
        /// settings and the values to inject.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub prompts: Vec<SettingPrompt>,
    }
);

// EXTENDING

api_struct!(
    /// Input passed to the `extend_project_graph` function.
    pub struct ExtendProjectGraphInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace extension configuration.
        /// Is null when within toolchains.
        pub extension_config: serde_json::Value,

        /// Map of project IDs to their source location,
        /// relative from the workspace root.
        pub project_sources: BTreeMap<Id, String>,

        /// Workspace toolchain configuration.
        /// Is null when within extensions.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `extend_project_graph` function.
    #[serde(default)]
    pub struct ExtendProjectGraphOutput {
        /// Map of project IDs to extracted information in which to
        /// extend projects in the project graph.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub extended_projects: BTreeMap<Id, ExtendProjectOutput>,

        /// List of virtual files in which information was extracted from and
        /// should invalidate the project graph cache.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub input_files: Vec<PathBuf>,
    }
);

api_struct!(
    /// A project-to-project relationship.
    pub struct ProjectDependency {
        /// ID or alias of the depended on project.
        pub id: Id,

        /// Scope of the dependency relationship.
        pub scope: DependencyScope,

        /// Quick information on where the dependency came from.
        pub via: Option<String>,
    }
);

api_struct!(
    /// Output utilized within the `extend_project_graph` function.
    #[serde(default)]
    pub struct ExtendProjectOutput {
        /// A unique alias for this project, different from the moon ID,
        /// typically extracted from a manifest.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alias: Option<String>,

        /// List of implicit dependencies, typically extracted from a manifest.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub dependencies: Vec<ProjectDependency>,

        /// Map of inherited tasks keyed by a unique ID, typically extracted from a manifest.
        #[serde(skip_serializing_if = "BTreeMap::is_empty")]
        pub tasks: BTreeMap<Id, PartialTaskConfig>,
    }
);

// TASK

api_enum!(
    /// Type of extend/merge strategy.
    #[serde(tag = "strategy", content = "value")]
    pub enum Extend<T> {
        /// Empty the data.
        Empty,

        /// Append to the data.
        Append(T),

        /// Prepend to the data.
        Prepend(T),

        /// Replace the data.
        Replace(T),
    }
);

api_struct!(
    /// Input passed to the `extend_task_command` function.
    pub struct ExtendTaskCommandInput {
        /// The current arguments, after the command.
        pub args: Vec<String>,

        /// Current moon context.
        pub context: MoonContext,

        /// The current command (binary/program).
        pub command: String,

        /// Workspace extension configuration.
        /// Is null when within toolchains.
        pub extension_config: serde_json::Value,

        /// Virtual path to a global executables directory
        /// for the current toolchain. Is null when within extensions.
        pub globals_dir: Option<VirtualPath>,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// Fragment of the owning task.
        pub task: TaskFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence. Is null when
        /// within extensions.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `extend_task_command` function.
    #[serde(default)]
    pub struct ExtendTaskCommandOutput {
        /// The command (binary/program) to use. Will replace the existing
        /// command. Can be overwritten by subsequent extend calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub command: Option<String>,

        /// List of arguments to merge with.
        /// Can be modified by subsequent extend calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub args: Option<Extend<Vec<String>>>,

        /// Map of environment variables to add.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub env: FxHashMap<String, String>,

        /// List of environment variables to remove.
        /// Can be overwritten by subsequent extend calls.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env_remove: Vec<String>,

        /// List of absolute paths to prepend into the `PATH` environment
        /// variable, but after the proto prepended paths. These *must*
        /// be real paths, not virtual!
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub paths: Vec<PathBuf>,
    }
);

api_struct!(
    /// Input passed to the `extend_task_script` function.
    pub struct ExtendTaskScriptInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace extension configuration.
        /// Is null when within toolchains.
        pub extension_config: serde_json::Value,

        /// Virtual path to a global executables directory
        /// for the current toolchain. Is null when within extensions.
        pub globals_dir: Option<VirtualPath>,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// The current script.
        pub script: String,

        /// Fragment of the owning task.
        pub task: TaskFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence. Is null when
        /// within extensions.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `extend_task_script` function.
    #[serde(default)]
    pub struct ExtendTaskScriptOutput {
        /// Map of environment variables to add.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub env: FxHashMap<String, String>,

        /// List of environment variables to remove.
        /// Can be overwritten by subsequent extend calls.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub env_remove: Vec<String>,

        /// List of absolute paths to prepend into the `PATH` environment
        /// variable, but after the proto prepended paths.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub paths: Vec<PathBuf>,

        /// The script to use. Will replace the existing script.
        /// Can be overwritten by subsequent extend calls.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub script: Option<String>,
    }
);

// SYNC WORKSPACE / PROJECT

api_struct!(
    /// Input passed to the `sync_workspace` function.
    pub struct SyncWorkspaceInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace extension configuration.
        /// Is null when within toolchains.
        pub extension_config: serde_json::Value,

        /// Workspace toolchain configuration.
        /// Is null when within extensions.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Input passed to the `sync_project` function.
    pub struct SyncProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Workspace extension configuration.
        /// Is null when within toolchains.
        pub extension_config: serde_json::Value,

        /// Other projects that the project being synced depends on.
        pub project_dependencies: Vec<ProjectFragment>,

        /// Fragment of the project being synced.
        pub project: ProjectFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence. Is null when within extensions.
        pub toolchain_config: serde_json::Value,

        /// Workspace only toolchain configuration.
        pub toolchain_workspace_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `sync_workspace` and `sync_project` functions.
    #[serde(default)]
    pub struct SyncOutput {
        /// List of files that have been changed because of the sync.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<PathBuf>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations: Vec<Operation>,

        /// Whether the action was skipped or not.
        #[serde(skip_serializing_if = "is_false")]
        pub skipped: bool,
    }
);
