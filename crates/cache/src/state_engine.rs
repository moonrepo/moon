use crate::resolve_path;
use moon_cache_item::CacheItem;
use moon_common::path::encode_component;
use moon_target::{Target, TargetProjectScope, TargetTaskScope};
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_utils::{fs, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug)]
pub struct StateEngine {
    /// The `.moon/cache/states` directory. Stores state information about anything...
    /// tools, dependencies, projects, tasks, etc.
    pub states_dir: PathBuf,
}

impl StateEngine {
    pub fn new(cache_dir: &Path) -> miette::Result<StateEngine> {
        let states_dir = cache_dir.join("states");

        debug!(
            states_dir = ?states_dir,
            "Creating states engine",
        );

        fs::create_dir_all(&states_dir)?;

        Ok(StateEngine { states_dir })
    }

    pub fn get_project_dir(&self, project_id: &str) -> PathBuf {
        self.states_dir.join(encode_component(project_id))
    }

    pub fn get_project_snapshot_path(&self, project_id: &str) -> PathBuf {
        self.get_project_dir(project_id).join("snapshot.json")
    }

    pub fn get_tag_dir(&self, tag: &str) -> PathBuf {
        self.states_dir
            .join(format!("tag-{}", encode_component(tag)))
    }

    pub fn get_task_dir(&self, project_id: &str, task_id: &str) -> PathBuf {
        self.get_project_dir(project_id)
            .join(encode_component(task_id))
    }

    pub fn get_target_dir(&self, target: &Target) -> PathBuf {
        let (scope, value) = target.get_task_scope();

        let name = match scope {
            TargetTaskScope::Id => value.to_string(),
            TargetTaskScope::Tag => format!("tag-{value}"),
        };

        let (scope, value) = target.get_project_scope();

        let dir = match scope {
            TargetProjectScope::Id => self.get_project_dir(value),
            TargetProjectScope::Tag => self.get_tag_dir(value),
            _ => self.get_project_dir("_"),
        };

        dir.join(encode_component(name))
    }

    pub fn load_state<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.resolve_path(path))
    }

    pub fn load_target_state<T>(&self, target: &Target) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.get_target_dir(target).join("lastRun.json"))
    }

    pub fn save_project_snapshot<T>(&self, project_id: &str, data: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path = self.get_project_snapshot_path(project_id);

        debug!(cache = ?path, "Writing project snapshot");

        // This purposefully ignores the cache mode and always writes!
        json::write_file(path, &data, false)?;

        Ok(())
    }

    pub fn resolve_path(&self, path: impl AsRef<OsStr>) -> PathBuf {
        resolve_path(&self.states_dir, path)
    }
}
