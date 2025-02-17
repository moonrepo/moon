use crate::common::*;
use moon_common::Id;
use schematic::Schema;
use warpgate_api::{api_struct, VirtualPath};

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

// SYNC WORKSPACE

api_struct!(
    /// Input passed to the `sync_workspace` function.
    pub struct SyncWorkspaceInput {
        /// Current moon context.
        pub context: MoonContext,
    }
);

api_struct!(
    /// Output returned from the `sync_workspace` function.
    pub struct SyncWorkspaceOutput {
        /// Operations to perform.
        pub operations: Vec<Operation>,
    }
);

// SYNC PROJECT

api_struct!(
    /// Input passed to the `sync_project` function.
    pub struct SyncProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Other project IDs that the project being synced depends on.
        pub project_dependencies: Vec<Id>,

        /// ID of the project being synced.
        pub project_id: Id,
    }
);

api_struct!(
    /// Output returned from the `sync_project` function.
    pub struct SyncProjectOutput {
        /// List of files that have been changed because of the sync action.
        pub changed_files: Vec<VirtualPath>,
    }
);
