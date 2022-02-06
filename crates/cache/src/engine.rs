use crate::items::{CacheItem, TargetRunState, WorkspaceState};
use moon_error::{map_io_to_fs_error, MoonError};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    pub dir: PathBuf,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(".moon/cache");

        create_dir_all(&dir).map_err(|e| map_io_to_fs_error(e, dir.to_path_buf()))?;

        Ok(CacheEngine { dir })
    }

    pub async fn delete_runfiles(&self) -> Result<(), MoonError> {
        let dir = self.dir.join("runfiles");

        remove_dir_all(&dir).map_err(|e| map_io_to_fs_error(e, dir.to_path_buf()))?;

        Ok(())
    }

    pub async fn runfile<'de, T: DeserializeOwned + Serialize>(
        &self,
        hash: &str,
        data: T,
    ) -> Result<CacheItem<T>, MoonError> {
        let path: PathBuf = ["runfiles", &format!("{}.json", hash)].iter().collect();

        Ok(CacheItem::load(self.dir.join(path), data).await?)
    }

    pub async fn run_target_state(
        &self,
        target: &str,
    ) -> Result<CacheItem<TargetRunState>, MoonError> {
        let path: PathBuf = ["runs", &target.replace(':', "/"), "lastState.json"]
            .iter()
            .collect();

        Ok(CacheItem::load(
            self.dir.join(path),
            TargetRunState {
                target: String::from(target),
                ..TargetRunState::default()
            },
        )
        .await?)
    }

    pub async fn workspace_state(&self) -> Result<CacheItem<WorkspaceState>, MoonError> {
        Ok(CacheItem::load(
            self.dir.join("workspaceState.json"),
            WorkspaceState::default(),
        )
        .await?)
    }
}
