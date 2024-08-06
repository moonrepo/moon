use crate::MoonContext;
use moon_common::Id;
use moon_config::{DependencyConfig, ProjectConfig};
use rustc_hash::FxHashMap;
use schematic::Schema;
use warpgate_api::{api_struct, VirtualPath};

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
        pub config_schema: Option<Schema>,
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

// SYNC PROJECT

api_struct!(
    pub struct SyncProjectRecord {
        pub config: ProjectConfig,
        pub dependencies: Vec<DependencyConfig>,
        pub id: Id,
        pub root: VirtualPath,
        pub source: String,
    }
);

api_struct!(
    /// Input passed to the `sync_project` function.
    pub struct SyncProjectInput {
        /// Current moon context.
        pub context: MoonContext,

        /// Other projects that the project being synced depends on.
        pub dependencies: FxHashMap<Id, SyncProjectRecord>,

        /// The project being synced.
        pub project: SyncProjectRecord,
    }
);
