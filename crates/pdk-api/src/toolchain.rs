use crate::common::*;
use moon_project::ProjectFragment;
use moon_task::TaskFragment;
use schematic::Schema;
use warpgate_api::{VirtualPath, api_struct};

// METADATA

api_struct!(
    /// Input passed to the `register_toolchain` function.
    pub struct RegisterToolchainInput {
        /// ID of the toolchain, as it was configured.
        pub id: String,
    }
);

api_struct!(
    /// Output returned from the `register_toolchain` function.
    pub struct RegisterToolchainOutput {
        /// Schema shape of the tool's configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub config_schema: Option<Schema>,

        /// Name of the toolchain.
        pub name: String,

        /// Optional description about what the toolchain does.
        pub description: Option<String>,

        /// Version of the plugin.
        pub plugin_version: String,
    }
);

// SYNC WORKSPACE / PROJECT

api_struct!(
    /// Input passed to the `sync_workspace` function.
    pub struct SyncWorkspaceInput {
        /// Current moon context.
        pub context: MoonContext,
    }
);

api_struct!(
    /// Input passed to the `sync_project` function.
    pub struct SyncProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Other projects that the project being synced depends on.
        pub project_dependencies: Vec<ProjectFragment>,

        /// Fragment of the project being synced.
        pub project: ProjectFragment,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `sync_workspace` and `sync_project` functions.
    #[serde(default)]
    pub struct SyncOutput {
        /// List of files that have been changed because of the sync.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub changed_files: Vec<VirtualPath>,

        /// Operations that were performed. This can be used to track
        /// metadata like time taken, result status, and more.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub operations_performed: Vec<Operation>,

        /// Whether the action was skipped or not.
        pub skipped: bool,
    }
);

// RUN TASK

api_struct!(
    /// Input passed to the `hash_task_contents` function.
    pub struct HashTaskContentsInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Fragment of the project that the task belongs to.
        pub project: ProjectFragment,

        /// Fragment of the task being hashed.
        pub task: TaskFragment,

        /// Merged toolchain configuration.
        pub toolchain_config: serde_json::Value,
    }
);

api_struct!(
    /// Output returned from the `hash_task_contents` function.
    pub struct HashTaskContentsOutput {
        /// Contents that should be included during hash generation.
        pub contents: Vec<serde_json::Value>,
    }
);
