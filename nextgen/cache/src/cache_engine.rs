use moon_cache_item::*;
use moon_common::consts;
use moon_time::parse_duration;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub cache_dir: PathBuf,

    /// The `.moon/cache/states` directory. Stores state information about anything...
    /// tools, dependencies, projects, tasks, etc.
    pub states_dir: PathBuf,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> miette::Result<CacheEngine> {
        let dir = workspace_root.join(consts::CONFIG_DIRNAME).join("cache");
        let states_dir = dir.join("states");
        let cache_tag = dir.join("CACHEDIR.TAG");

        debug!(
            cache_dir = ?dir,
            states_dir = ?states_dir,
            "Creating cache engine",
        );

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
            cache_dir: dir,
            states_dir,
        })
    }

    pub fn cache<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.resolve_path(path))
    }

    pub fn cache_state<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        self.cache(self.states_dir.join(path.as_ref()))
    }

    pub fn clean_stale_cache(&self, lifetime: &str) -> miette::Result<(usize, u64)> {
        let duration =
            parse_duration(lifetime).map_err(|e| miette::miette!("Invalid lifetime: {e}"))?;

        debug!(
            "Cleaning up and deleting stale cache older than \"{}\"",
            lifetime
        );

        let stats = fs::remove_dir_stale_contents(&self.cache_dir, duration)?;
        let deleted = stats.files_deleted;
        let bytes = stats.bytes_saved;

        debug!("Deleted {} files and saved {} bytes", deleted, bytes);

        Ok((deleted, bytes))
    }

    pub fn get_mode(&self) -> CacheMode {
        get_cache_mode()
    }

    pub fn write<T>(&self, path: impl AsRef<OsStr>, data: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path = self.resolve_path(path);

        debug!(cache = ?path, "Writing cache");

        // This purposefully ignores the cache mode and always writes!
        json::write_file(path, &data, false)?;

        Ok(())
    }

    pub fn write_state<T>(&self, path: impl AsRef<OsStr>, state: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.write(self.states_dir.join(path.as_ref()), state)
    }

    fn resolve_path(&self, path: impl AsRef<OsStr>) -> PathBuf {
        let path = PathBuf::from(path.as_ref());

        if path.is_absolute() {
            path
        } else {
            self.cache_dir.join(path)
        }
    }
}
