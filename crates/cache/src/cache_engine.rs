use crate::{HashEngine, StateEngine, merge_clean_results, resolve_path};
use moon_cache_item::*;
use moon_common::consts;
use moon_common::path::encode_component;
use moon_env_var::GlobalEnvBag;
use moon_time::parse_duration;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_utils::fs::{FileLock, RemoveDirContentsResult};
use starbase_utils::{fs, json};
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub cache_dir: PathBuf,

    /// Manages reading and writing of content hashable items.
    pub hash: HashEngine,

    /// Manages states of projects, tasks, tools, and more.
    pub state: StateEngine,

    /// A temporary directory for random artifacts.
    pub temp_dir: PathBuf,

    mode: CacheMode,
    forced_mode: RwLock<Option<CacheMode>>,
}

impl CacheEngine {
    pub fn new(workspace_root: &Path) -> miette::Result<CacheEngine> {
        let dir = workspace_root.join(consts::CONFIG_DIRNAME).join("cache");
        let cache_tag = dir.join("CACHEDIR.TAG");

        debug!(
            cache_dir = ?dir,
            "Creating cache engine",
        );

        fs::create_dir_all(&dir)?;

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
            hash: HashEngine::new(&dir)?,
            state: StateEngine::new(&dir)?,
            temp_dir: dir.join("temp"),
            cache_dir: dir,
            mode: get_cache_mode(),
            forced_mode: RwLock::new(None),
        })
    }

    pub fn force_mode(&self, mode: CacheMode) {
        let _ = self.forced_mode.write().unwrap().insert(mode);

        GlobalEnvBag::instance().set("MOON_CACHE", mode.to_string());
    }

    pub fn cache<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.resolve_path(path))
    }

    #[instrument(skip(self))]
    pub fn clean_stale_cache(&self, lifetime: &str, all: bool) -> miette::Result<(usize, u64)> {
        let duration = self.parse_lifetime(lifetime)?;

        debug!(
            "Cleaning up and deleting stale cached artifacts older than \"{}\"",
            lifetime
        );

        let mut dirs = vec![&self.hash.hashes_dir, &self.hash.outputs_dir];

        if all {
            dirs.push(&self.state.states_dir);
            dirs.push(&self.temp_dir);
        }

        let mut result = RemoveDirContentsResult {
            files_deleted: 0,
            bytes_saved: 0,
        };

        for dir in dirs {
            result = merge_clean_results(result, fs::remove_dir_stale_contents(dir, duration)?);
        }

        debug!(
            "Deleted {} artifacts and saved {} bytes",
            result.files_deleted, result.bytes_saved
        );

        Ok((result.files_deleted, result.bytes_saved))
    }

    pub fn create_lock<T: AsRef<str>>(&self, name: T) -> miette::Result<FileLock> {
        let mut name = encode_component(name.as_ref());

        if !name.ends_with(".lock") {
            name.push_str(".lock");
        }

        let guard = fs::lock_file(self.cache_dir.join("locks").join(name))?;

        Ok(guard)
    }

    pub fn write<K, T>(&self, path: K, data: &T) -> miette::Result<()>
    where
        K: AsRef<OsStr>,
        T: ?Sized + Serialize,
    {
        let path = self.resolve_path(path);

        debug!(cache = ?path, "Writing cache");

        // This purposefully ignores the cache mode and always writes!
        json::write_file(path, &data, false)?;

        Ok(())
    }

    pub async fn execute_if_changed<K, T, F, Fut>(
        &self,
        path: K,
        data: T,
        op: F,
    ) -> miette::Result<bool>
    where
        K: AsRef<OsStr>,
        T: Serialize,
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<bool>> + Send,
    {
        let path = self.resolve_path(path);
        let name = fs::file_name(&path);

        let mut state = self.state.load_state::<CommonCacheState>(&name)?;
        let hash = self.hash.save_manifest_without_hasher(&name, data)?;

        if hash != state.data.last_hash {
            let result = op().await?;

            state.data.last_hash = hash;
            state.save()?;

            return Ok(result);
        }

        Ok(false)
    }

    pub fn parse_lifetime(&self, lifetime: &str) -> miette::Result<Duration> {
        parse_duration(lifetime).map_err(|error| miette::miette!("Invalid lifetime: {error}"))
    }

    pub fn resolve_path(&self, path: impl AsRef<OsStr>) -> PathBuf {
        resolve_path(&self.cache_dir, path)
    }

    pub fn is_readable(&self) -> bool {
        self.get_mode().is_readable()
    }

    pub fn is_read_only(&self) -> bool {
        self.get_mode().is_read_only()
    }

    pub fn is_writable(&self) -> bool {
        self.get_mode().is_writable()
    }

    pub fn is_write_only(&self) -> bool {
        self.get_mode().is_write_only()
    }

    fn get_mode(&self) -> CacheMode {
        if let Ok(lock) = self.forced_mode.read() {
            if let Some(mode) = &*lock {
                return *mode;
            }
        }

        self.mode
    }
}
