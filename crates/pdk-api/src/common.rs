use crate::context::MoonContext;
use moon_common::Id;
use moon_config::{PartialDependencyConfig, PartialTaskConfig};
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;
use warpgate_api::*;

// EXTENDING

api_struct!(
    /// Input passed to the `extend_project_graph` function.
    pub struct ExtendProjectGraphInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Map of project IDs to their source location,
        /// relative from the workspace root.
        pub project_sources: BTreeMap<Id, String>,
    }
);

api_struct!(
    /// Output returned from the `extend_project_graph` function.
    pub struct ExtendProjectGraphOutput {
        /// Map of project IDs to their alias (typically from a manifest).
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        pub project_aliases: BTreeMap<Id, String>,
    }
);

api_struct!(
    /// Input passed to the `extend_project` function.
    pub struct ExtendProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Fragment of the project to extend.
        pub project: ProjectFragment,
    }
);

api_struct!(
    /// Output returned from the `extend_project` function.
    #[serde(default)]
    pub struct ExtendProjectOutput {
        /// A custom alias to be used alongside the project ID.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alias: Option<String>,

        /// Map of implicit dependencies, keyed by their alias, typically extracted from a manifest.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub dependencies: FxHashMap<String, PartialDependencyConfig>,

        /// Map of inherited tasks, typically extracted from a manifest.
        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub tasks: FxHashMap<Id, PartialTaskConfig>,
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

        /// Virtual path to a global executables directory
        /// for the current toolchain.
        pub globals_dir: Option<VirtualPath>,

        /// Fragment of the owning task.
        pub task: TaskFragment,
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
        /// variable, but after the proto prepended paths.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub paths: Vec<PathBuf>,
    }
);

api_struct!(
    /// Input passed to the `extend_task_script` function.
    pub struct ExtendTaskScriptInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Virtual path to a global executables directory
        /// for the current toolchain.
        pub globals_dir: Option<VirtualPath>,

        /// The current script.
        pub script: String,

        /// Fragment of the owning task.
        pub task: TaskFragment,
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
    }
);
