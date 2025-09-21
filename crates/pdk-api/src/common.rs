use crate::context::MoonContext;
use moon_common::Id;
use moon_config::{DependencyScope, PartialTaskConfig};
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

        /// Workspace toolchain configuration.
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

        /// Virtual path to a global executables directory
        /// for the current toolchain.
        pub globals_dir: Option<VirtualPath>,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// Fragment of the owning task.
        pub task: TaskFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
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

        /// Virtual path to a global executables directory
        /// for the current toolchain.
        pub globals_dir: Option<VirtualPath>,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// The current script.
        pub script: String,

        /// Fragment of the owning task.
        pub task: TaskFragment,

        /// Workspace and project merged toolchain configuration,
        /// with the latter taking precedence.
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
