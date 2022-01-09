use crate::errors::CacheError;
use crate::items::CacheItem;
use moon_utils::string_vec;
use serde_json::to_string;
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct CacheEngine {
    /// The `.moon/cache` dir relative to workspace root.
    root: PathBuf,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> Result<Self, CacheError> {
        let root = workspace_root.join(".moon/cache");

        create_dir_all(&root)?;

        Ok(CacheEngine { root })
    }

    pub async fn read(&self, _item: CacheItem) {}

    pub async fn write(&self, item: CacheItem) -> Result<(), CacheError> {
        Ok(fs::write(
            self.get_item_cache_path(&item),
            self.stringify_item_value(&item)?,
        )
        .await?)
    }

    fn get_item_cache_path(&self, item: &CacheItem) -> PathBuf {
        let parts = match item {
            CacheItem::RunTarget(i) => {
                string_vec![target_to_path(i.target.as_str()), "lastRun.json"]
            }
        };

        let cache_path: PathBuf = parts.iter().collect();

        self.root.join(cache_path)
    }

    fn stringify_item_value(&self, item: &CacheItem) -> Result<String, CacheError> {
        match item {
            CacheItem::RunTarget(i) => Ok(to_string(&i)?),
        }
    }
}

fn target_to_path(target: &str) -> String {
    target.replace(':', "/")
}
