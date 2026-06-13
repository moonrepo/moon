//! Serializable interface for source-control provider plugins.
//!
//! # Lifecycle
//!
//! A host registers a provider and can detect whether it applies to the
//! workspace. moon then calls `initialize_vcs` once when it creates the VCS
//! client for a command. Initialization occurs before any impact or hook query.
//! A new initialization requires a new plugin instance.
//!
//! The provider retains any opaque state needed to pin the initialized state.
//! Every later operation must answer from that state, including after a cache miss;
//! it must not silently refresh the working copy or re-resolve movable labels
//! against newer repository state.

use crate::{Id, MoonContext, VirtualPath};
use bitflags::bitflags;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use warpgate_api::{api_enum, api_struct, api_unit_enum};

/// Exact wire-protocol generation supported by this host or provider.
///
/// VCS plugins use lockstep protocol generations. Adding fields without safe
/// serde defaults, changing lifecycle semantics, or assigning new change-mask
/// bits requires incrementing this version.
pub const VCS_PLUGIN_PROTOCOL_VERSION: u16 = 6;

api_struct!(
    /// Opaque provider-defined identity for an exact repository state.
    #[serde(transparent)]
    pub struct VcsStateId(pub String);
);

impl VcsStateId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for VcsStateId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for VcsStateId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for VcsStateId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for VcsStateId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

api_struct!(
    /// Opaque provider expression that resolves to a repository state.
    #[serde(transparent)]
    pub struct VcsReference(pub String);
);

impl VcsReference {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl AsRef<str> for VcsReference {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for VcsReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<String> for VcsReference {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for VcsReference {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<VcsStateId> for VcsReference {
    fn from(value: VcsStateId) -> Self {
        Self(value.into_inner())
    }
}

impl From<&VcsStateId> for VcsReference {
    fn from(value: &VcsStateId) -> Self {
        Self(value.as_str().to_owned())
    }
}

api_struct!(
    /// Input passed to `register_vcs` before any other provider operation.
    pub struct RegisterVcsInput {
        /// ID under which the host loaded this plugin instance.
        pub id: Id,
        /// Exact protocol generation required by the host.
        pub host_protocol_version: u16,
    }
);

impl VcsState {
    pub fn id_str(&self) -> Option<&str> {
        self.id.as_ref().map(VcsStateId::as_str)
    }

    pub fn id_reference(&self) -> Option<VcsReference> {
        self.id.as_ref().map(VcsReference::from)
    }
}

api_struct!(
    /// Provider metadata returned by `register_vcs`.
    #[serde(default)]
    pub struct RegisterVcsOutput {
        /// Human-readable provider name.
        pub name: String,
        /// Optional human-readable provider description.
        pub description: Option<String>,
        /// Version of the provider implementation, independent of the protocol.
        pub plugin_version: String,
        /// Exact protocol generation implemented by the provider.
        pub protocol_version: u16,
    }
);

api_struct!(
    /// Repository roots found by `detect_vcs` and fixed by `initialize_vcs`.
    pub struct VcsRoots {
        /// Root of the repository metadata.
        pub repository_root: VirtualPath,
        /// Root of the active worktree or working copy.
        pub working_root: VirtualPath,
    }
);

api_struct!(
    /// Input passed to `detect_vcs` before the VCS client is initialized.
    pub struct DetectVcsInput {
        pub context: MoonContext,
    }
);

api_struct!(
    /// Applicability reported by `detect_vcs`.
    ///
    /// `roots` are present when this VCS client applies to the workspace. User
    /// configuration controls whether a plugin is selected at all.
    #[serde(default)]
    pub struct DetectVcsOutput {
        /// Roots are present when this client applies to the workspace.
        pub roots: Option<VcsRoots>,
        /// Human-readable explanation of the detection result.
        pub reason: String,
    }
);

api_struct!(
    /// Input passed to `initialize_vcs` exactly once per plugin instance.
    pub struct InitializeVcsInput {
        /// Movable provider expression to resolve and pin as the baseline.
        pub baseline: Option<VcsReference>,
        /// Preferred remote names, in priority order, for repository metadata.
        #[serde(default)]
        pub remote_candidates: Vec<String>,
        pub context: MoonContext,
    }
);

api_struct!(
    /// An exact provider state resolved during initialization.
    pub struct VcsState {
        /// Exact state identity, or `None` when the repository has no recorded
        /// state yet, such as an unborn Git repository.
        ///
        /// The ID must remain stable and round-trippable to the provider for the
        /// lifetime of this initialization. It need not survive rewritten history
        /// or a later plugin instance.
        pub id: Option<VcsStateId>,
        /// Human-readable bookmark, branch, channel, change, or equivalent.
        /// Labels may move and must not be used as exact state identities.
        pub label: Option<String>,
    }
);

api_unit_enum!(
    /// Availability of repository history at initialization.
    pub enum VcsHistoryCompleteness {
        /// All history required for comparisons is available.
        Complete,
        /// History is known to be incomplete, such as in a shallow clone.
        Incomplete,
        /// The provider cannot determine whether history is complete.
        #[default]
        Unknown,
    }
);

api_struct!(
    /// Provider metadata and exact states pinned by `initialize_vcs`.
    pub struct InitializeVcsOutput {
        /// Stable VCS client kind, such as `git` or `jj`.
        pub client: Id,
        /// Version of the source-control client used by the provider.
        pub client_version: Option<String>,
        /// Repository roots fixed by this initialization.
        pub roots: VcsRoots,
        /// Current state as it existed during initialization.
        pub current: VcsState,
        /// Current recorded state, excluding working changes.
        ///
        /// This may equal `current`, as in Git, or identify its recorded parent,
        /// as with a Jujutsu working-copy commit. Providers may synthesize an
        /// initialization-scoped state when multiple parents must be represented.
        pub recorded: VcsState,
        /// Baseline resolved from `InitializeVcsInput::baseline`, when available.
        pub baseline: Option<VcsState>,
        pub repository_slug: Option<String>,
        pub history: VcsHistoryCompleteness,
    }
);

api_enum!(
    /// moon-level reason for requesting impacted files.
    #[derive(Default)]
    #[serde(tag = "type", rename_all = "kebab-case")]
    pub enum VcsImpactIntent {
        /// Working changes captured during initialization.
        #[default]
        Working,
        /// Changes introduced by `head` since it diverged from `base`.
        ///
        /// When `base` is absent, compare `head` with its provider-defined
        /// predecessor. When `head` is absent, use the initialized recorded state.
        /// Movable expressions must be resolved from the initialized state.
        Submission {
            base: Option<VcsReference>,
            head: Option<VcsReference>,
            /// Include working changes captured during initialization.
            include_working: bool,
        },
    }
);

api_struct!(
    /// Input passed to `get_vcs_impacts` after initialization.
    pub struct GetVcsImpactsInput {
        /// Must be identical to the initialization context.
        pub context: MoonContext,
        pub intent: VcsImpactIntent,
    }
);

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[cfg_attr(feature = "schematic", derive(schematic::Schematic))]
#[serde(transparent)]
pub struct VcsChangeMask(u8);

bitflags! {
    /// Compact set of change-kind and location flags for one path.
    ///
    /// A valid mask contains at least one change-kind bit and at least one
    /// location bit. Unassigned bits are reserved: receiving one is a protocol
    /// error, and assigning a new bit requires a protocol-version increment.
    impl VcsChangeMask: u8 {
    /// The path was added.
    const ADDED = 1;
    /// The path was deleted.
    const DELETED = 2;
    /// The path was modified.
    const MODIFIED = 4;
    /// The path changed in recorded history.
    const RECORDED = 8;
    /// The path changed in a staging area.
    const STAGED = 16;
    /// The path changed in the working copy.
    const WORKING = 32;
    /// The path is not tracked by the provider.
    const UNTRACKED = 64;

    const CHANGE_BITS = Self::ADDED.bits() | Self::DELETED.bits() | Self::MODIFIED.bits();
    const LOCATION_BITS = Self::RECORDED.bits() | Self::STAGED.bits() | Self::WORKING.bits() | Self::UNTRACKED.bits();
    const KNOWN_BITS = Self::CHANGE_BITS.bits() | Self::LOCATION_BITS.bits();
    }
}

api_unit_enum!(
    /// Safety guarantee attached to an impact result.
    pub enum VcsImpactCompleteness {
        /// Every impacted path and applicable mask bit is present exactly.
        Exact,
        /// Every possibly impacted path and mask bit is present, but the result
        /// may contain false positives. False negatives are forbidden.
        Conservative,
        /// The provider could not produce a safe answer.
        #[default]
        Unavailable,
    }
);

api_struct!(
    /// Output returned by `get_vcs_impacts`.
    #[serde(default)]
    pub struct GetVcsImpactsOutput {
        /// Canonical UTF-8 file paths relative to the moon workspace root.
        ///
        /// Keys use `/` separators and must be non-empty, must not contain `.`,
        /// `..`, empty components, backslashes, NULs, or names invalid on the
        /// host platform, and must not be absolute.
        pub changes: BTreeMap<PathBuf, VcsChangeMask>,
        pub completeness: VcsImpactCompleteness,
        /// Human-readable explanations for degraded or unavailable results.
        pub diagnostics: Vec<String>,
    }
);

api_struct!(
    /// Input passed to `setup_vcs_hook_environment` after initialization.
    pub struct SetupVcsHookEnvironmentInput {
        /// Must be identical to the initialization context.
        pub context: MoonContext,
        /// Canonical path to moon's hooks directory.
        pub hooks_dir: VirtualPath,
        /// Provider-native hook names that moon intends to install.
        pub hooks: Vec<String>,
    }
);

api_struct!(
    /// Hook execution environment returned by `setup_vcs_hook_environment`.
    #[serde(default)]
    pub struct SetupVcsHookEnvironmentOutput {
        /// Working directory in which moon should execute installed hooks.
        pub working_dir: Option<VirtualPath>,
    }
);

api_struct!(
    /// Input passed to `teardown_vcs_hook_environment` after initialization.
    pub struct TeardownVcsHookEnvironmentInput {
        /// Must be identical to the initialization context.
        pub context: MoonContext,
        /// Canonical path to moon's hooks directory.
        pub hooks_dir: VirtualPath,
        /// Provider-native hook names previously managed by moon.
        pub hooks: Vec<String>,
    }
);

api_struct!(
    /// Output returned by `teardown_vcs_hook_environment`.
    pub struct TeardownVcsHookEnvironmentOutput {}
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_working_intents() {
        assert_eq!(
            serde_json::to_value(VcsImpactIntent::Working).unwrap(),
            serde_json::json!({"type": "working"})
        );
        assert_eq!(
            serde_json::to_value(VcsImpactIntent::Submission {
                base: None,
                head: None,
                include_working: true,
            })
            .unwrap(),
            serde_json::json!({
                "type": "submission",
                "base": null,
                "head": null,
                "include_working": true,
            })
        );
    }

    #[test]
    fn serializes_vcs_identifiers_as_strings() {
        let output = InitializeVcsOutput {
            client: Id::raw("git"),
            client_version: Some("2.0.0".into()),
            roots: VcsRoots {
                repository_root: VirtualPath::new("/repo/.git"),
                working_root: VirtualPath::new("/repo"),
            },
            current: VcsState {
                id: Some(VcsStateId::from("abc123")),
                label: Some("main".into()),
            },
            recorded: VcsState {
                id: None,
                label: None,
            },
            baseline: None,
            repository_slug: None,
            history: VcsHistoryCompleteness::Complete,
        };

        let value = serde_json::to_value(output).unwrap();

        assert_eq!(value["client"], serde_json::json!("git"));
        assert_eq!(value["current"]["id"], serde_json::json!("abc123"));
        assert_eq!(value["recorded"]["id"], serde_json::Value::Null);
        assert_eq!(
            serde_json::to_value(VcsReference::from("main")).unwrap(),
            serde_json::json!("main")
        );
    }

    #[test]
    fn serializes_change_masks_as_numbers() {
        assert_eq!(
            serde_json::to_value(VcsChangeMask::ADDED).unwrap(),
            serde_json::json!(1)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::DELETED).unwrap(),
            serde_json::json!(2)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::MODIFIED).unwrap(),
            serde_json::json!(4)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::RECORDED).unwrap(),
            serde_json::json!(8)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::STAGED).unwrap(),
            serde_json::json!(16)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::WORKING).unwrap(),
            serde_json::json!(32)
        );
        assert_eq!(
            serde_json::to_value(VcsChangeMask::UNTRACKED).unwrap(),
            serde_json::json!(64)
        );

        let output = GetVcsImpactsOutput {
            changes: BTreeMap::from([
                (
                    PathBuf::from("a.txt"),
                    VcsChangeMask::ADDED | VcsChangeMask::WORKING,
                ),
                (
                    PathBuf::from("z.txt"),
                    VcsChangeMask::MODIFIED | VcsChangeMask::RECORDED,
                ),
            ]),
            completeness: VcsImpactCompleteness::Exact,
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(output).unwrap(),
            serde_json::json!({
                "changes": {
                    "a.txt": 33,
                    "z.txt": 12,
                },
                "completeness": "exact",
                "diagnostics": [],
            })
        );
    }

    #[test]
    fn defaults_missing_impact_completeness_to_unavailable() {
        let output: GetVcsImpactsOutput = serde_json::from_value(serde_json::json!({
            "changes": {},
            "diagnostics": [],
        }))
        .unwrap();

        assert_eq!(output.completeness, VcsImpactCompleteness::Unavailable);
    }
}
