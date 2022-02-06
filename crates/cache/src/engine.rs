use crate::items::{CacheItem, TargetRunState, WorkspaceState};
use crate::runfiles::CacheRunfile;
use moon_error::MoonError;
use moon_utils::fs::{create_dir_all, remove_dir_all};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    pub dir: PathBuf,
}

impl CacheEngine {
    pub async fn new(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(".moon/cache");

        create_dir_all(&dir).await?;

        Ok(CacheEngine { dir })
    }

    pub async fn delete_runfiles(&self) -> Result<(), MoonError> {
        remove_dir_all(&self.dir.join("runfiles")).await?;

        Ok(())
    }

    pub async fn runfile<T: DeserializeOwned + Serialize>(
        &self,
        path: &str,
        id: &str,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        let path: PathBuf = ["runfiles", path, &format!("{}.json", id)].iter().collect();

        Ok(CacheRunfile::load(self.dir.join(path), data).await?)
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
