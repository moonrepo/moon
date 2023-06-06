use crate::hash_error::HashError;
use crate::hasher::ContentHasher;
use serde::Serialize;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

pub struct HashEngine {
    /// The `.moon/cache/hashes` directory. Stores hash manifests.
    pub hashes_dir: PathBuf,

    /// The `.moon/cache/outputs` directory. Stores task outputs as hashed archives.
    pub outputs_dir: PathBuf,
}

impl HashEngine {
    pub fn new(cache_dir: &Path) -> Result<HashEngine, HashError> {
        let hashes_dir = cache_dir.join("hashes");
        let outputs_dir = cache_dir.join("outputs");

        debug!(
            hashes_dir = %hashes_dir.display(),
            outputs_dir = %outputs_dir.display(),
            "Creating hash engine",
        );

        Ok(HashEngine {
            hashes_dir,
            outputs_dir,
        })
    }

    pub fn save_manifest<T>(&self, mut contents: ContentHasher<T>) -> Result<String, HashError>
    where
        T: Serialize,
    {
        let hash = contents.generate()?;
        let path = self.hashes_dir.join(format!("{hash}.json"));

        debug!(manifest = %path.display(), "Saving hash manifest");

        fs::write_file(&path, contents.serialize()?)?;

        Ok(hash)
    }
}
