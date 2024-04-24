use crate::{merge_clean_results, resolve_path, HashEngine, StateEngine};
use moon_cache_item::*;
use moon_common::consts;
use moon_time::parse_duration;
use serde::de::DeserializeOwned;
use serde::Serialize;
use starbase_utils::{fs, json};
use std::ffi::OsStr;
use std::future::Future;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct CacheEngine {
    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub cache_dir: PathBuf,

    /// Manages reading and writing of content hashable items.
    pub hash: HashEngine,

    /// Manages states of projects, tasks, tools, and more.
    pub state: StateEngine,
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
            cache_dir: dir,
        })
    }

    pub fn cache<T>(&self, path: impl AsRef<OsStr>) -> miette::Result<CacheItem<T>>
    where
        T: Default + DeserializeOwned + Serialize,
    {
        CacheItem::<T>::load(self.resolve_path(path))
    }

    pub fn clean_stale_cache(&self, lifetime: &str, all: bool) -> miette::Result<(usize, u64)> {
        let duration = parse_duration(lifetime)
            .map_err(|error| miette::miette!("Invalid lifetime: {error}"))?;

        debug!(
            "Cleaning up and deleting stale cached artifacts older than \"{}\"",
            lifetime
        );

        let result = if all {
            merge_clean_results(
                self.state.clean_stale_cache(duration)?,
                self.hash.clean_stale_cache(duration)?,
            )
        } else {
            self.hash.clean_stale_cache(duration)?
        };

        debug!(
            "Deleted {} artifacts and saved {} bytes",
            result.files_deleted, result.bytes_saved
        );

        Ok((result.files_deleted, result.bytes_saved))
    }

    pub fn get_mode(&self) -> CacheMode {
        get_cache_mode()
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
    ) -> miette::Result<()>
    where
        K: AsRef<OsStr>,
        T: Serialize,
        F: FnOnce() -> Fut,
        Fut: Future<Output = miette::Result<()>> + Send,
    {
        let path = self.resolve_path(path);
        let name = fs::file_name(&path);

        let mut state = self.state.load_state::<CommonState>(&name)?;
        let hash = self.hash.save_manifest_without_hasher(&name, data)?;

        if hash != state.data.last_hash {
            op().await?;

            state.data.last_hash = hash;
            state.save()?;
        }

        Ok(())
    }

    pub fn resolve_path(&self, path: impl AsRef<OsStr>) -> PathBuf {
        resolve_path(&self.cache_dir, path)
    }
}
