use crate::helpers::LOG_TARGET;
use crate::items::{DependenciesState, ProjectsState, RunTargetState, ToolState};
use crate::runfiles::Snapshot;
use crate::{get_cache_mode, CacheMode};
use moon_common::consts::CONFIG_DIRNAME;
use moon_logger::{debug, trace};
use moon_platform_runtime::Runtime;
use moon_utils::time;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_styles::color;
use starbase_utils::{fs, json};
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};

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
    pub fn load(workspace_root: &Path) -> miette::Result<Self> {
        let dir = workspace_root.join(CONFIG_DIRNAME).join("cache");
        let hashes_dir = dir.join("hashes");
        let outputs_dir = dir.join("outputs");
        let states_dir = dir.join("states");
        let cache_tag = dir.join("CACHEDIR.TAG");

        debug!(
            target: LOG_TARGET,
            "Creating cache engine at {}",
            color::path(&dir)
        );

        // Do this once instead of each time we are writing cache items
        fs::create_dir_all(&hashes_dir)?;
        fs::create_dir_all(&outputs_dir)?;
        fs::create_dir_all(&states_dir)?;

        // Create a cache directory tag
        if !cache_tag.exists() {
            fs::write_file(
                cache_tag,
                r#"Signature: 8a477f597d28d172789f06886806bc55
# This file is a cache directory tag created by moon.
# For information see https://bford.info/cachedir"#,
            )?;
        }

        Ok(CacheEngine {
            dir,
            hashes_dir,
            outputs_dir,
            states_dir,
        })
    }

    pub fn cache_deps_state(
        &self,
        runtime: &Runtime,
        project_id: Option<&str>,
    ) -> miette::Result<DependenciesState> {
        let name = format!("deps{runtime}.json");

        DependenciesState::load(self.get_state_path(if let Some(id) = project_id {
            format!("{id}{}{name}", MAIN_SEPARATOR_STR)
        } else {
            name
        }))
    }

    pub fn cache_run_target_state<T: AsRef<str>>(
        &self,
        target_id: T,
    ) -> miette::Result<RunTargetState> {
        let target_id = target_id.as_ref();
        let mut item = RunTargetState::load(self.get_target_dir(target_id).join("lastRun.json"))?;

        if item.target.is_empty() {
            item.target = target_id.to_owned();
        }

        Ok(item)
    }

    // pub fn cache_codeowners_state(&self) -> miette::Result<CommonState> {
    //     CommonState::load(self.get_state_path("codeowners.json"))
    // }

    pub fn cache_projects_state(&self) -> miette::Result<ProjectsState> {
        ProjectsState::load(self.get_state_path("projects.json"))
    }

    pub fn cache_tool_state(&self, runtime: &Runtime) -> miette::Result<ToolState> {
        ToolState::load(self.get_state_path(format!("tool{}-{}.json", runtime, runtime.version())))
    }

    // pub fn cache_vcs_hooks_state(&self) -> miette::Result<CommonState> {
    //     CommonState::load(self.get_state_path("vcsHooks.json"))
    // }

    pub fn clean_stale_cache(&self, lifetime: &str) -> miette::Result<(usize, u64)> {
        let duration =
            time::parse_duration(lifetime).map_err(|e| miette::miette!("Invalid lifetime: {e}"))?;

        trace!(
            target: LOG_TARGET,
            "Cleaning up and deleting stale cache older than \"{}\"",
            lifetime
        );

        let hashes_dir = fs::remove_dir_stale_contents(&self.hashes_dir, duration)?;
        let outputs_dir = fs::remove_dir_stale_contents(&self.outputs_dir, duration)?;

        let deleted = hashes_dir.files_deleted + outputs_dir.files_deleted;
        let bytes = hashes_dir.bytes_saved + outputs_dir.bytes_saved;

        trace!(
            target: LOG_TARGET,
            "Deleted {} files and saved {} bytes",
            deleted,
            bytes
        );

        Ok((deleted, bytes))
    }

    pub fn create_hash_manifest<T>(&self, hash: &str, contents: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path = self.get_hash_manifest_path(hash);

        trace!(
            target: LOG_TARGET,
            "Writing hash manifest {}",
            color::path(&path)
        );

        json::write_file(&path, &contents, true)?;

        Ok(())
    }

    pub fn create_json_report<T: Serialize>(&self, name: &str, data: T) -> miette::Result<()> {
        let path = self.dir.join(name);

        trace!(target: LOG_TARGET, "Writing report {}", color::path(&path));

        json::write_file(path, &data, true)?;

        Ok(())
    }

    pub fn create_snapshot<T: DeserializeOwned + Serialize>(
        &self,
        project_id: &str,
        data: &T,
    ) -> miette::Result<Snapshot> {
        Snapshot::load(self.get_state_path(project_id).join("snapshot.json"), data)
    }

    pub fn get_hash_archive_path(&self, hash: &str) -> PathBuf {
        self.outputs_dir.join(format!("{hash}.tar.gz"))
    }

    pub fn get_hash_manifest_path(&self, hash: &str) -> PathBuf {
        self.hashes_dir.join(format!("{hash}.json"))
    }

    pub fn get_mode(&self) -> CacheMode {
        get_cache_mode()
    }

    pub fn get_state_path<T: AsRef<str>>(&self, file: T) -> PathBuf {
        self.states_dir.join(file.as_ref())
    }

    pub fn get_target_dir<T: AsRef<str>>(&self, target_id: T) -> PathBuf {
        self.get_state_path(target_id.as_ref().replace(':', MAIN_SEPARATOR_STR))
    }
}
