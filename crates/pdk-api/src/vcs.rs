//! Serializable interface for source-control provider plugins.

use crate::{Id, MoonContext};
use warpgate_api::{api_enum, api_struct, api_unit_enum};

pub const VCS_PLUGIN_PROTOCOL_VERSION: u16 = 1;

api_struct!(
    pub struct RegisterVcsInput {
        pub id: Id,
        pub host_protocol_version: u16,
    }
);

api_struct!(
    #[serde(default)]
    pub struct VcsPluginMetadata {
        pub name: String,
        pub description: Option<String>,
        pub plugin_version: String,
        pub protocol_version: u16,
        pub supports_hooks: bool,
    }
);

api_struct!(
    pub struct DetectVcsInput {
        pub context: MoonContext,
    }
);

api_struct!(
    #[serde(default)]
    pub struct DetectVcsOutput {
        pub active: bool,
        pub reason: String,
    }
);

api_unit_enum!(
    pub enum VcsConsistency {
        ExistingObservation,
        #[default]
        FreshObservation,
    }
);

api_struct!(
    pub struct ObserveVcsInput {
        pub baseline: Option<String>,
        #[serde(default)]
        pub remote_candidates: Vec<String>,
        pub consistency: VcsConsistency,
        pub context: MoonContext,
    }
);

api_struct!(
    pub struct VcsStateIdentity {
        /// Provider-defined stable identity for this state.
        pub id: String,
        /// Human-readable bookmark, branch, channel, change, or equivalent.
        pub label: Option<String>,
    }
);

api_unit_enum!(
    pub enum VcsHistoryCompleteness {
        Complete,
        Incomplete,
        #[default]
        Unknown,
    }
);

api_struct!(
    pub struct VcsObservation {
        /// Opaque provider-defined token that pins later queries.
        pub id: String,
        pub provider: String,
        pub client_version: Option<String>,
        pub enabled: bool,
        pub repository_root: String,
        pub working_root: String,
        pub current: VcsStateIdentity,
        pub baseline: Option<VcsStateIdentity>,
        pub repository_slug: Option<String>,
        pub history: VcsHistoryCompleteness,
    }
);

api_enum!(
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum VcsImpactIntent {
        #[default]
        Workspace,
        Submission {
            base: Option<String>,
            head: Option<String>,
            include_workspace: bool,
        },
    }
);

api_struct!(
    pub struct GetVcsImpactsInput {
        pub context: MoonContext,
        pub observation_id: String,
        pub intent: VcsImpactIntent,
    }
);

api_unit_enum!(
    pub enum VcsChangeLayer {
        #[default]
        Recorded,
        Staged,
        Workspace,
        Untracked,
    }
);

api_struct!(
    pub struct VcsPathEffect {
        pub before: Option<String>,
        pub after: Option<String>,
        pub layers: Vec<VcsChangeLayer>,
    }
);

api_unit_enum!(
    pub enum VcsImpactCompleteness {
        #[default]
        Exact,
        Conservative,
        Unavailable,
    }
);

api_struct!(
    #[serde(default)]
    pub struct GetVcsImpactsOutput {
        pub effects: Vec<VcsPathEffect>,
        pub completeness: VcsImpactCompleteness,
        pub diagnostics: Vec<String>,
    }
);

api_struct!(
    pub struct SetupVcsHooksInput {
        pub context: MoonContext,
        pub observation_id: String,
        pub hooks_dir: String,
    }
);

api_struct!(
    #[serde(default)]
    pub struct SetupVcsHooksOutput {
        pub hooks_dir: Option<String>,
        pub working_dir: Option<String>,
    }
);

api_struct!(
    pub struct TeardownVcsHooksInput {
        pub context: MoonContext,
        pub observation_id: String,
    }
);

api_struct!(
    #[serde(default)]
    pub struct TeardownVcsHooksOutput {
        pub removed: bool,
    }
);
