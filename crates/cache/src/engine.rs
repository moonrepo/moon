use crate::helpers::{is_writable, LOG_TARGET};
use crate::items::{CacheItem, ProjectsState, RunTargetState, WorkspaceState};
use crate::runfiles::CacheRunfile;
use moon_archive::tar;
use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_logger::{color, debug, trace};
use moon_utils::fs;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub dir: PathBuf,

    /// The `.moon/cache/hashes` directory. Stores hash manifests.
    pub hashes_dir: PathBuf,

    /// The `.moon/cache/runs` directory. Stores run states and runfiles.
    pub runs_dir: PathBuf,

    /// The `.moon/cache/out` directory. Stores task outputs as hashed archives.
    pub outputs_dir: PathBuf,
}

impl CacheEngine {
    pub async fn create(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(CONFIG_DIRNAME).join("cache");
        let hashes_dir = dir.join("hashes");
        let runs_dir = dir.join("runs");
        let outputs_dir = dir.join("out");

        debug!(
            target: LOG_TARGET,
            "Creating cache engine at {}",
            color::path(&dir)
        );

        fs::create_dir_all(&hashes_dir).await?;
        fs::create_dir_all(&runs_dir).await?;
        fs::create_dir_all(&outputs_dir).await?;

        Ok(CacheEngine {
            dir,
            hashes_dir,
            runs_dir,
            outputs_dir,
        })
    }

    pub async fn cache_run_target_state(
        &self,
        target_id: &str,
    ) -> Result<CacheItem<RunTargetState>, MoonError> {
        CacheItem::load(
            self.get_target_dir(target_id).join("lastRunState.json"),
            RunTargetState {
                target: String::from(target_id),
                ..RunTargetState::default()
            },
            0,
        )
        .await
    }

    pub async fn cache_projects_state(&self) -> Result<CacheItem<ProjectsState>, MoonError> {
        CacheItem::load(
            self.dir.join("projectsState.json"),
            ProjectsState::default(),
            90000, // Cache for 3 minutes
        )
        .await
    }

    pub async fn cache_workspace_state(&self) -> Result<CacheItem<WorkspaceState>, MoonError> {
        CacheItem::load(
            self.dir.join("workspaceState.json"),
            WorkspaceState::default(),
            0,
        )
        .await
    }

    pub async fn create_hash_archive(
        &self,
        hash: &str,
        project_root: &Path,
        outputs: &[String],
    ) -> Result<(), MoonError> {
        if is_writable() {
            // Old implementation would copy files to a hashed folder,
            // so if we encounter that folder, let's just remove it!
            let old_hash_folder = self.outputs_dir.join(hash);

            if old_hash_folder.exists() && old_hash_folder.is_dir() {
                fs::remove_dir_all(old_hash_folder).await?;
            }

            // New implementation uses tar archives! Very cool.
            tar(
                project_root,
                outputs,
                self.get_hash_archive_path(hash),
                None,
            )?;

            trace!(
                target: LOG_TARGET,
                "Archiving outputs for {}",
                color::symbol(hash)
            );
        }

        Ok(())
    }

    pub async fn create_hash_manifest<T>(&self, hash: &str, hasher: &T) -> Result<(), MoonError>
    where
        T: ?Sized + Serialize,
    {
        if is_writable() {
            let path = self.get_hash_manifest_path(hash);

            trace!(
                target: LOG_TARGET,
                "Writing hash manifest {}",
                color::path(&path)
            );

            fs::write_json(&path, &hasher, true).await?;
        }

        Ok(())
    }

    pub async fn create_runfile<T: DeserializeOwned + Serialize>(
        &self,
        project_id: &str,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        CacheRunfile::load(self.get_project_dir(project_id).join("runfile.json"), data).await
    }

    pub async fn delete_hash(&self, hash: &str) -> Result<(), MoonError> {
        if is_writable() {
            trace!(target: LOG_TARGET, "Deleting hash {}", color::symbol(hash));

            fs::remove_file(self.get_hash_manifest_path(hash)).await?;
            fs::remove_file(self.get_hash_archive_path(hash)).await?;
        }

        Ok(())
    }

    pub fn get_hash_archive_path(&self, hash: &str) -> PathBuf {
        self.outputs_dir.join(format!("{}.tar.gz", hash))
    }

    pub fn get_hash_manifest_path(&self, hash: &str) -> PathBuf {
        self.hashes_dir.join(format!("{}.json", hash))
    }

    pub fn get_project_dir(&self, project_id: &str) -> PathBuf {
        self.runs_dir.join(project_id)
    }

    pub fn get_target_dir(&self, target_id: &str) -> PathBuf {
        let path: PathBuf = [&target_id.replace(':', "/")].iter().collect();

        self.runs_dir.join(path)
    }
}
