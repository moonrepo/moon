use crate::errors::CacheError;
use crate::items::{CacheItem, WorkspaceStateItem};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    root: PathBuf,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> Result<Self, CacheError> {
        let root = workspace_root.join(".moon/cache");

        create_dir_all(&root)?;

        Ok(CacheEngine { root })
    }

    pub fn to_millis(&self, time: SystemTime) -> u128 {
        match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => d.as_millis(),
            Err(_) => 0,
        }
    }

    pub async fn workspace_state(&self) -> Result<CacheItem<WorkspaceStateItem>, CacheError> {
        Ok(CacheItem::load(
            self.root.join("workspaceState.json"),
            WorkspaceStateItem::default(),
        )
        .await?)
    }
}

fn target_to_path(target: &str) -> String {
    target.replace(':', "/")
}
