use crate::context::MoonContext;
use moon_common::Id;
use std::collections::BTreeMap;
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
