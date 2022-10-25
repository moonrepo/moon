use crate::helpers::{is_writable, LOG_TARGET};
use crate::items::{DependenciesState, ProjectsState, RunTargetState, ToolState};
use crate::runfiles::CacheRunfile;
use moon_constants::CONFIG_DIRNAME;
use moon_error::MoonError;
use moon_logger::{color, debug, trace};
use moon_platform::Runtime;
use moon_utils::{fs, time};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub dir: PathBuf,

    /// The `.moon/cache/hashes` directory. Stores hash manifests.
    pub hashes_dir: PathBuf,

    /// The `.moon/cache/outputs` directory. Stores task outputs as hashed archives.
    pub outputs_dir: PathBuf,

    /// The `.moon/cache/states` directory. Stores state information about anything...
    /// tools, dependencies, projects, tasks, etc.
    pub states_dir: PathBuf,
}

impl CacheEngine {
    pub async fn load(workspace_root: &Path) -> Result<Self, MoonError> {
        let dir = workspace_root.join(CONFIG_DIRNAME).join("cache");
        let hashes_dir = dir.join("hashes");
        let out_dir = dir.join("out");
        let outputs_dir = dir.join("outputs");
        let states_dir = dir.join("states");

        debug!(
            target: LOG_TARGET,
            "Creating cache engine at {}",
            color::path(&dir)
        );

        // TODO: Remove in v1. This was renamed from out -> outputs,
        // but we didn't want to lose existing cache.
        if out_dir.exists() {
            let _ = std::fs::rename(out_dir, &outputs_dir);
        }

        // Do this once instead of each time we are writing cache items
        fs::create_dir_all(&hashes_dir).await?;
        fs::create_dir_all(&outputs_dir).await?;
        fs::create_dir_all(&states_dir).await?;

        Ok(CacheEngine {
            dir,
            hashes_dir,
            outputs_dir,
            states_dir,
        })
    }

    pub async fn cache_deps_state(
        &self,
        runtime: &Runtime,
        project_id: Option<&str>,
    ) -> Result<DependenciesState, MoonError> {
        let name = format!("deps{}.json", runtime);

        DependenciesState::load(
            self.states_dir.join(if let Some(id) = project_id {
                format!("{}/{}", id, name)
            } else {
                name
            }),
            0,
        )
        .await
    }

    pub async fn cache_run_target_state(
        &self,
        target_id: &str,
    ) -> Result<RunTargetState, MoonError> {
        let mut item =
            RunTargetState::load(self.get_target_dir(target_id).join("lastRun.json"), 0).await?;

        if item.target.is_empty() {
            item.target = target_id.to_owned();
        }

        Ok(item)
    }

    pub async fn cache_projects_state(&self) -> Result<ProjectsState, MoonError> {
        ProjectsState::load(
            self.states_dir.join("projects.json"),
            90000, // Cache for 3 minutes
        )
        .await
    }

    pub async fn cache_tool_state(&self, runtime: &Runtime) -> Result<ToolState, MoonError> {
        ToolState::load(
            self.states_dir
                .join(format!("tool{}-{}.json", runtime, runtime.version())),
            0,
        )
        .await
    }

    pub async fn clean_stale_cache(
        &self,
        lifetime: &str,
    ) -> Result<fs::RemoveDirContentsResult, MoonError> {
        let duration = time::parse_duration(lifetime)
            .map_err(|e| MoonError::Generic(format!("Invalid lifetime: {}", e)))?;

        trace!(
            target: LOG_TARGET,
            "Cleaning up and deleting stale cache older than \"{}\"",
            lifetime
        );

        let (hashes_deleted, hashes_bytes) =
            fs::remove_dir_stale_contents(&self.hashes_dir, duration).await?;

        let (outputs_deleted, outputs_bytes) =
            fs::remove_dir_stale_contents(&self.outputs_dir, duration).await?;

        let deleted = hashes_deleted + outputs_deleted;
        let bytes = hashes_bytes + outputs_bytes;

        trace!(
            target: LOG_TARGET,
            "Deleted {} files and saved {} bytes",
            deleted,
            bytes
        );

        Ok((deleted, bytes))
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

    pub async fn create_json_report<T: Serialize>(
        &self,
        name: &str,
        data: T,
    ) -> Result<(), MoonError> {
        let path = self.dir.join(name);

        trace!(target: LOG_TARGET, "Writing report {}", color::path(&path));

        fs::write_json(path, &data, true).await?;

        Ok(())
    }

    pub async fn create_runfile<T: DeserializeOwned + Serialize>(
        &self,
        project_id: &str,
        data: &T,
    ) -> Result<CacheRunfile, MoonError> {
        CacheRunfile::load(self.states_dir.join(project_id).join("runfile.json"), data).await
    }

    pub fn get_hash_archive_path(&self, hash: &str) -> PathBuf {
        self.outputs_dir.join(format!("{}.tar.gz", hash))
    }

    pub fn get_hash_manifest_path(&self, hash: &str) -> PathBuf {
        self.hashes_dir.join(format!("{}.json", hash))
    }

    pub fn get_target_dir(&self, target_id: &str) -> PathBuf {
        self.states_dir.join(target_id.replace(':', "/"))
    }
}
