use moon_action::Operation;
use moon_cache_item::cache_item;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::{ContentHash, Digest};
use std::collections::BTreeMap;

cache_item!(
    pub struct TaskRunCacheState {
        pub exit_code: i32,
        pub hash: String,
        pub last_run_time: u128,
        pub target: String,

        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        pub output_hashes: BTreeMap<WorkspaceRelativePathBuf, ContentHash>,
    }
);

#[derive(Debug, Default)]
pub struct TaskRunState {
    /// The bytes of our internal fingerprint.
    pub bytes: Vec<u8>,

    /// The digest of our internal fingerprint. This is separate from the action
    /// digest as this implementation is not Bazel compatible.
    pub digest: Digest,

    /// The last operation that was executed.
    pub operation: Operation,
}
