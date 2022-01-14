use crate::errors::CacheError;
use crate::items::{CacheItem, TargetRunState, WorkspaceState};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

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

    pub async fn target_run_state(
        &self,
        target: &str,
    ) -> Result<CacheItem<TargetRunState>, CacheError> {
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

    pub async fn workspace_state(&self) -> Result<CacheItem<WorkspaceState>, CacheError> {
        Ok(CacheItem::load(
            self.root.join("workspaceState.json"),
            WorkspaceState::default(),
        )
        .await?)
    }
}
