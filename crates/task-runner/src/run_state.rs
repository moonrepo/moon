use moon_cache_item::cache_item;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_hash::ContentHash;
use moon_remote::RemoteDigest;
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

pub struct TaskRunState {
    pub action_digest: RemoteDigest,
    pub action_bytes: Vec<u8>,
}
