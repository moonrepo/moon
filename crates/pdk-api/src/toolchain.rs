use moon_common::Id;
use moon_config::{DependencyConfig, ProjectConfig};
use rustc_hash::FxHashMap;
use warpgate_api::{api_struct, VirtualPath};

pub use crate::MoonContext;

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
