use crate::output_tree::OutputDigestsMap;
use bazel_remote_apis::build::bazel::remote::execution::v2::Action;
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

// This is where moon differs from the Bazel RE API. In Bazel,
// we would serialize + hash the `Action` and `Command` types,
// and upload those. But those types do not match how our hashing
// works, so instead, we're uploading the bytes of our internal
// hash manifests. Hopefully this doesn't cause issues!

#[derive(Debug, Default)]
pub struct TaskRunState {
    // The digest of our internal fingerprint. This is separate from the action
    // digest as this implementation is not Bazel compatible.
    pub digest: Digest,

    /// The last operation that was executed, which may be used to resume an incomplete run.
    pub operation: Operation,

    pub output_digests: OutputDigestsMap,
}
