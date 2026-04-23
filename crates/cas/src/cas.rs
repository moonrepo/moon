use crate::cas_error::CasError;
use crate::config::CasStoreConfig;
use crate::content_hash::ContentHash;
use crate::fs::*;
use crate::gc::GcResult;
use starbase_utils::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument};
use uuid::Uuid;

/// A content-addressable file system store.
///
/// Content is addressed by its BLAKE3 hash and stored in a git-style
/// prefix-sharded directory layout. Writes are atomic (temp file + rename)
/// so readers never observe partial data, and no file locking is required.
#[derive(Debug)]
pub struct CasStore {
    pub root: PathBuf,
    pub objects_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub config: CasStoreConfig,
}

/// Drop guard that removes a temp file unless explicitly committed.
struct TempGuard {
    path: PathBuf,
    committed: bool,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

impl CasStore {
    /// Create or open a CAS store rooted at `root`.
    ///
    /// Creates the `v1/` and `temp/` subdirectories if they do not exist.
    pub fn new(root: impl AsRef<Path>, config: CasStoreConfig) -> miette::Result<Self> {
        let root = root.as_ref();
        let objects_dir = root.join("v1");
        let temp_dir = root.join("temp");

        debug!(root = ?root, "Opening CAS store");

        fs::create_dir_all(&objects_dir)?;
        fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            root: root.to_path_buf(),
            objects_dir,
            temp_dir,
            config,
        })
    }

    // ---- Write operations ----

    /// Store raw bytes. Returns the content hash. Idempotent: if the blob
    /// already exists, this is a no-op that returns the hash immediately.
    #[instrument(skip(self, bytes), fields(len = bytes.len()))]
    pub fn write_bytes(&self, bytes: &[u8]) -> miette::Result<ContentHash> {
        let hash = ContentHash::hash_bytes(bytes);

        if self.object_path(&hash).exists() {
            return Ok(hash);
        }

        let mut guard = self.create_temp_file()?;

        {
            let mut file = fs::create_file(&guard.path)?;

            file.write_all(bytes)
                .map_err(|error| CasError::WriteFailed {
                    path: guard.path.clone(),
                    error: Box::new(error),
                })?;

            file.sync_all().map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;
        }

        self.commit_temp_file(&hash, &mut guard)?;

        debug!(hash = %hash, "Stored blob from bytes");

        Ok(hash)
    }

    /// Store content read from a file at `source`. Uses memory-mapped I/O for
    /// hashing files larger than the configured threshold.
    #[instrument(skip(self))]
    pub fn write_file(&self, source: &Path) -> miette::Result<ContentHash> {
        let hash = ContentHash::hash_file(source, self.config.mmap_threshold)?;

        if self.object_path(&hash).exists() {
            return Ok(hash);
        }

        let mut guard = self.create_temp_file()?;

        fs::copy_file(source, &guard.path)?;

        {
            let file = fs::open_file(&guard.path)?;

            file.sync_all().map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;
        }

        self.commit_temp_file(&hash, &mut guard)?;

        debug!(hash = %hash, source = ?source, "Stored blob from file");

        Ok(hash)
    }

    /// Store content from a streaming reader. Hashes and writes simultaneously
    /// in 64 KiB chunks.
    pub fn write_stream<R: Read>(&self, mut reader: R) -> miette::Result<ContentHash> {
        let mut guard = self.create_temp_file()?;

        let hash = {
            let mut hasher = blake3::Hasher::new();
            let mut file = fs::create_file(&guard.path)?;
            let mut buffer = [0u8; 64 * 1024];

            loop {
                let n = reader
                    .read(&mut buffer)
                    .map_err(|error| CasError::ReadFailed {
                        path: guard.path.clone(),
                        error: Box::new(error),
                    })?;

                if n == 0 {
                    break;
                }

                hasher.update(&buffer[..n]);

                file.write_all(&buffer[..n])
                    .map_err(|error| CasError::WriteFailed {
                        path: guard.path.clone(),
                        error: Box::new(error),
                    })?;
            }

            file.sync_all().map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;

            ContentHash::from_hash(hasher.finalize())
        };

        if self.object_path(&hash).exists() {
            return Ok(hash);
        }

        self.commit_temp_file(&hash, &mut guard)?;

        debug!(hash = %hash, "Stored blob from byte stream");

        Ok(hash)
    }

    // ---- Read operations ----

    /// Check whether a blob exists for the given hash.
    pub fn contains_blob(&self, hash: &ContentHash) -> bool {
        self.object_path(hash).exists()
    }

    /// Read the full blob into memory. Verifies integrity if configured.
    #[instrument(skip(self))]
    pub fn read_bytes(&self, hash: &ContentHash) -> miette::Result<Vec<u8>> {
        let path = self.object_path_with_exists_check(hash)?;
        let bytes = fs::read_file_bytes(&path)?;

        if self.config.verify_on_read {
            self.verify_integrity(&path, hash, &bytes)?;
        }

        Ok(bytes)
    }

    /// Open the blob as a [`std::fs::File`] handle for streaming reads.
    /// Verifies integrity before returning the handle if configured.
    pub fn open(&self, hash: &ContentHash) -> miette::Result<std::fs::File> {
        let path = self.object_path_with_exists_check(hash)?;

        if self.config.verify_on_read {
            let bytes = fs::read_file_bytes(&path)?;
            self.verify_integrity(&path, hash, &bytes)?;
        }

        std::fs::File::open(&path).map_err(|error| {
            CasError::ReadFailed {
                path,
                error: Box::new(error),
            }
            .into()
        })
    }

    // ---- Hard-link operations ----

    /// Hard-link a CAS blob to `dest`. Falls back to copy if hard-linking
    /// fails (e.g. cross-device).
    #[instrument(skip(self))]
    pub fn link_to(&self, hash: &ContentHash, dest: &Path) -> miette::Result<()> {
        let source = self.object_path_with_exists_check(hash)?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Remove existing file at destination if present, since hard-linking
        // requires the destination to not exist
        fs::remove_file(dest)?;

        match std::fs::hard_link(&source, dest) {
            Ok(_) => {}
            Err(_) => {
                fs::copy_file(&source, dest)?;
            }
        };

        debug!(hash = %hash, dest = ?dest, "Linked blob to destination");

        Ok(())
    }

    /// Ingest a file into the store by hard-linking it. Falls back to
    /// `write_file` if hard-linking fails. Returns the content hash.
    #[instrument(skip(self))]
    pub fn link_from(&self, source: &Path) -> miette::Result<ContentHash> {
        let hash = ContentHash::hash_file(source, self.config.mmap_threshold)?;
        let object_path = self.object_path(&hash);

        if object_path.exists() {
            return Ok(hash);
        }

        self.ensure_shard_dir(&hash)?;

        match std::fs::hard_link(source, &object_path) {
            Ok(_) => {
                mark_readonly(&object_path)?;
            }
            Err(_) => {
                // Cross-device or permission issue; fall back to copy via write_file.
                return self.write_file(source);
            }
        }

        debug!(hash = %hash, source = ?source, "Linked blob from source");

        Ok(hash)
    }

    // ---- Lifecycle ----

    /// Remove blobs whose mtime is older than `max_age`.
    #[instrument(skip(self))]
    pub async fn gc(&self, max_age: Duration) -> miette::Result<GcResult> {
        crate::gc::gc(self, max_age).await
    }

    /// Remove all blobs from the store.
    pub async fn purge(&self) -> miette::Result<GcResult> {
        crate::gc::purge(self).await
    }

    /// Update a blob's mtime to now, keeping it alive through GC.
    pub fn touch(&self, hash: &ContentHash) -> miette::Result<()> {
        let path = self.object_path_with_exists_check(hash)?;

        // Temporarily make writable so we can update mtime
        mark_writable(&path)?;

        let file = fs::open_file(&path)?;

        file.set_modified(SystemTime::now())
            .map_err(|error| CasError::WriteFailed {
                path: path.clone(),
                error: Box::new(error),
            })?;

        drop(file);

        // Restore read-only
        mark_readonly(&path)?;

        Ok(())
    }

    // ---- Internal helpers ----

    pub fn object_path(&self, hash: &ContentHash) -> PathBuf {
        self.objects_dir.join(hash.prefix()).join(hash.suffix())
    }

    pub fn object_path_with_exists_check(&self, hash: &ContentHash) -> miette::Result<PathBuf> {
        let path = self.object_path(hash);

        if !path.exists() {
            return Err(CasError::NotFound {
                hash: hash.to_string(),
            }
            .into());
        }

        Ok(path)
    }

    fn ensure_shard_dir(&self, hash: &ContentHash) -> miette::Result<()> {
        fs::create_dir_all(self.objects_dir.join(hash.prefix()))?;

        Ok(())
    }

    fn create_temp_file(&self) -> miette::Result<TempGuard> {
        let path = self.temp_dir.join(Uuid::new_v4().to_string());

        Ok(TempGuard {
            path,
            committed: false,
        })
    }

    fn commit_temp_file(&self, hash: &ContentHash, guard: &mut TempGuard) -> miette::Result<()> {
        self.ensure_shard_dir(hash)?;

        let dest = self.object_path(hash);

        fs::rename(&guard.path, &dest)?;

        guard.committed = true;

        mark_readonly(&dest)?;

        Ok(())
    }

    fn verify_integrity(
        &self,
        path: &Path,
        expected: &ContentHash,
        bytes: &[u8],
    ) -> miette::Result<()> {
        let actual = blake3::hash(bytes).to_hex().to_string();

        if actual != expected.as_hex() {
            return Err(CasError::IntegrityMismatch {
                path: path.to_owned(),
                expected: expected.to_string(),
                actual,
            }
            .into());
        }

        Ok(())
    }
}
