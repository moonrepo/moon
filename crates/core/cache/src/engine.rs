use moon_common::consts::CONFIG_DIRNAME;
use serde::Serialize;
use starbase_utils::{fs, json};
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
    pub fn load(workspace_root: &Path) -> miette::Result<Self> {
        let dir = workspace_root.join(CONFIG_DIRNAME).join("cache");
        let hashes_dir = dir.join("hashes");
        let outputs_dir = dir.join("outputs");
        let states_dir = dir.join("states");

        // Do this once instead of each time we are writing cache items
        fs::create_dir_all(&hashes_dir)?;
        fs::create_dir_all(&outputs_dir)?;
        fs::create_dir_all(&states_dir)?;

        Ok(CacheEngine {
            dir,
            hashes_dir,
            outputs_dir,
            states_dir,
        })
    }

    pub fn create_hash_manifest<T>(&self, hash: &str, contents: &T) -> miette::Result<()>
    where
        T: ?Sized + Serialize,
    {
        let path = self.hashes_dir.join(format!("{hash}.json"));

        json::write_file(&path, &contents, true)?;

        Ok(())
    }
}
