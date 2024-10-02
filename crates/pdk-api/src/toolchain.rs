use crate::common::*;
use schematic::Schema;
use warpgate_api::api_struct;

// METADATA

api_struct!(
    /// Input passed to the `register_toolchain` function.
    pub struct ToolchainMetadataInput {
        /// ID of the toolchain, as it was configured.
        pub id: String,
    }
);

api_struct!(
    /// Output returned from the `register_toolchain` function.
    pub struct ToolchainMetadataOutput {
        /// Schema shape of the tool's configuration.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub config_schema: Option<Schema>,

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

// api_struct!(
//     /// Input passed to the `sync_project` function.
//     pub struct SyncProjectInput {
//         /// Current moon context.
//         pub context: MoonContext,

//         /// Other projects that the project being synced depends on.
//         // pub dependencies: FxHashMap<Id, SyncProjectRecord>,
//     }
// );
