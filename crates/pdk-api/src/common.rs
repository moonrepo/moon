use crate::context::MoonContext;
use moon_common::Id;
use moon_config::{PartialDependencyConfig, PartialTaskConfig};
use moon_project::ProjectFragment;
use rustc_hash::FxHashMap;
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
