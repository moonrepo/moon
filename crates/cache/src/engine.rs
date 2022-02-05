use crate::items::{CacheItem, TargetRunState, WorkspaceState};
use moon_error::{map_io_to_fs_error, MoonError};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    root: PathBuf,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> Result<Self, MoonError> {
        let root = workspace_root.join(".moon/cache");

        create_dir_all(&root).map_err(|e| map_io_to_fs_error(e, root.to_path_buf()))?;

        Ok(CacheEngine { root })
    }

    pub async fn run_target_state(
        &self,
        target: &str,
    ) -> Result<CacheItem<TargetRunState>, MoonError> {
        let path: PathBuf = ["runs", &target.replace(':', "/"), "lastState.json"]
            .iter()
            .collect();

        Ok(CacheItem::load(
            self.root.join(path),
            TargetRunState {
                target: String::from(target),
                ..TargetRunState::default()
            },
        )
        .await?)
    }

    pub async fn workspace_state(&self) -> Result<CacheItem<WorkspaceState>, MoonError> {
        Ok(CacheItem::load(
            self.root.join("workspaceState.json"),
            WorkspaceState::default(),
        )
        .await?)
    }
}
