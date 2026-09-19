use moon_hash::ContentHasher;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug)]
pub struct HashEngine {
    /// The `.moon/cache/hashes` directory. Stores hash manifests.
    pub hashes_dir: PathBuf,

    /// The `.moon/cache/outputs` directory. Stores task outputs as hashed archives.
    pub outputs_dir: PathBuf,
}

impl HashEngine {
    pub fn new(cache_dir: &Path) -> miette::Result<HashEngine> {
        let hashes_dir = cache_dir.join("hashes");
        let outputs_dir = cache_dir.join("outputs");

        debug!(
            hashes_dir = ?hashes_dir,
            outputs_dir = ?outputs_dir,
            "Creating hash engine",
        );

        fs::create_dir_all(&hashes_dir)?;
        fs::create_dir_all(&outputs_dir)?;

        Ok(HashEngine {
            hashes_dir,
            outputs_dir,
        })
    }

    pub fn create_hasher<T: AsRef<str>>(&self, label: T) -> ContentHasher {
        ContentHasher::new(label.as_ref())
    }

    pub fn get_archive_path(&self, hash: &str) -> PathBuf {
        self.outputs_dir.join(format!("{hash}.tar.gz"))
    }
}
