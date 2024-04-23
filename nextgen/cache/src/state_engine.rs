use moon_cache_item::CacheItem;
use moon_target::Target;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tracing::debug;

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

    pub fn get_path(&self, file: &str) -> PathBuf {
        let mut path = self.states_dir.join(file);
        path.set_extension("json");
        path
    }

    pub fn get_project_dir(&self, project_id: &str) -> PathBuf {
        self.states_dir.join(project_id)
    }

    pub fn get_project_snapshot_path(&self, project_id: &str) -> PathBuf {
        self.get_project_dir(project_id).join("snapshot.json")
    }

    pub fn get_project_task_dir(&self, project_id: &str, task_id: &str) -> PathBuf {
        self.get_project_dir(project_id).join(task_id)
    }

    pub fn get_target_dir(&self, target: &Target) -> PathBuf {
        self.get_project_task_dir(target.get_project_id().as_ref().unwrap(), &target.task_id)
    }

    pub fn load_state<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.resolve_path(path))
    }

    pub fn save_state<T>(&self, path: impl AsRef<OsStr>, data: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path = self.resolve_path(path);

        debug!(cache = ?path, "Writing state");

        // This purposefully ignores the cache mode and always writes!
        json::write_file(path, &data, false)?;

        Ok(())
    }

    fn resolve_path(&self, path: impl AsRef<OsStr>) -> PathBuf {
        let path = PathBuf::from(path.as_ref());

        if path.is_absolute() {
            path
        } else {
            self.states_dir.join(path)
        }
    }
}
