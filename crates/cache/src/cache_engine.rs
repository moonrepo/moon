use crate::hash_engine::HashEngine;
use crate::state_engine::StateEngine;
use crate::{merge_clean_results, resolve_path};
use miette::IntoDiagnostic;
use moon_cache_item::*;
use moon_cache_storage::{CacheContext, Storage};
use moon_cas::CasStore;
use moon_common::path::{WorkspaceRelativePathBuf, encode_component};
use moon_config::CacheCasConfig;
use moon_env_var::GlobalEnvBag;
use moon_hash::ContentHash;
use moon_time::parse_duration;
use serde::Serialize;
use serde::de::DeserializeOwned;
use starbase_utils::fs::{FileLock, RemoveDirContentsResult};
use starbase_utils::{fs, json};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct CacheEngine {
    /// A content-addressable storage of action results.
    pub ac: CasStore,

    /// The `.moon/cache` directory relative to workspace root.
    /// Contains cached items pertaining to runs and processes.
    pub cache_dir: PathBuf,

    /// A content-addressable storage of objects, primarily for
    /// storing task outputs.
    pub cas: CasStore,

    /// Manages reading and writing of content hashable items.
    pub hash: HashEngine,

    /// Manages states of projects, tasks, tools, and more.
    pub state: StateEngine,

    /// A content-addressable storage with multiple backends.
    pub storage: Storage,

    /// A temporary directory for random artifacts.
    pub temp_dir: PathBuf,

    #[allow(dead_code)]
    context: CacheContext,
    mode: CacheMode,
    forced_mode: RwLock<Option<CacheMode>>,
}

impl CacheEngine {
    pub fn new(context: CacheContext) -> miette::Result<CacheEngine> {
        let dir = &context.cache_dir;
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

        let hash = HashEngine::new(&dir)?;

        // Action cache always uses defaults
        let ac_config = CacheCasConfig::default();

        Ok(CacheEngine {
            ac: CasStore::new(dir.join("ac"), &ac_config)?,
            cas: CasStore::new(dir.join("cas"), &context.cache_config.cas)?,
            hash,
            state: StateEngine::new(&dir)?,
            storage: Storage::default(),
            temp_dir: dir.join("temp"),
            cache_dir: dir.to_owned(),
            mode: get_cache_mode(),
            forced_mode: RwLock::new(None),
            context,
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
    pub async fn clean_stale_cache(
        &self,
        lifetime: &str,
        all: bool,
    ) -> miette::Result<(usize, u64)> {
        let duration = self.parse_lifetime(lifetime)?;

        debug!(
            "Cleaning up and deleting stale cached artifacts older than \"{}\"",
            lifetime
        );

        let mut result = RemoveDirContentsResult {
            files_deleted: 0,
            bytes_saved: 0,
        };

        let locks_dir = self.cache_dir.join("locks");
        let mut dirs = vec![&self.hash.hashes_dir, &self.hash.outputs_dir, &locks_dir];

        if all {
            dirs.push(&self.state.states_dir);
            dirs.push(&self.temp_dir);
        }

        for dir in dirs {
            result = merge_clean_results(result, fs::remove_dir_stale_contents(dir, duration)?);
        }

        let ac_result = self.ac.gc(duration).await?;

        result.files_deleted += ac_result.blobs_removed;
        result.bytes_saved += ac_result.bytes_freed;

        let cas_result = self.cas.gc(duration).await?;

        result.files_deleted += cas_result.blobs_removed;
        result.bytes_saved += cas_result.bytes_freed;

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

        let mut guard = fs::lock_file(self.cache_dir.join("locks").join(name))?;
        guard.remove_on_unlock();

        Ok(guard)
    }

    pub async fn hash_files(
        &self,
        root: &Path,
        files: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        debug!("Hashing {} files", files.len());

        let mut map = BTreeMap::new();
        let mut set = JoinSet::<miette::Result<(WorkspaceRelativePathBuf, Option<String>)>>::new();
        // let mmap_threshold = self.config.cas.mmap_threshold;

        for file in files {
            let abs_file = file.to_logical_path(root);
            let rel_file = file.clone();

            if !abs_file.is_file() {
                continue;
            }

            set.spawn_blocking(move || {
                // File may have been deleted since we were given the path,
                // so check existence before hashing
                if !abs_file.exists() {
                    return Ok((rel_file, None));
                }

                let hash = ContentHash::hash_file(&abs_file)?;

                Ok((rel_file, Some(hash.to_string())))
            });
        }

        while let Some(result) = set.join_next().await {
            let (file, hash) = result.into_diagnostic()??;

            if let Some(hash) = hash {
                map.insert(file, hash);
            }
        }

        Ok(map)
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

    pub async fn execute_if_changed<K, T, F, R>(
        &self,
        label: K,
        fingerprint: T,
        op: F,
    ) -> miette::Result<Option<R>>
    where
        K: AsRef<str>,
        T: Serialize,
        F: AsyncFnOnce(&str) -> miette::Result<R>,
    {
        let mut hasher = self.hash.create_hasher(label.as_ref());
        hasher.hash_content(fingerprint)?;

        let hash = hasher.generate_hash()?;

        // If the hash manifest exists, then it has ran before,
        // otherwise run and write the manifest
        if !self.hash.get_manifest_path(&hash).exists() {
            let result = op(&hash).await?;

            self.hash.save_manifest(&mut hasher)?;

            return Ok(Some(result));
        }

        Ok(None)
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
        if let Ok(lock) = self.forced_mode.read()
            && let Some(mode) = &*lock
        {
            return *mode;
        }

        self.mode
    }
}
