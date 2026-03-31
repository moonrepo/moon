use crate::cas_error::CasError;
use crate::config::CasStoreConfig;
use crate::content_hash::ContentHash;
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
    root: PathBuf,
    objects_dir: PathBuf,
    tmp_dir: PathBuf,
    config: CasStoreConfig,
}

/// Drop guard that removes a temp file unless explicitly committed.
struct TempGuard {
    path: PathBuf,
    committed: bool,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

impl CasStore {
    /// Create or open a CAS store rooted at `root`.
    ///
    /// Creates the `objects/` and `tmp/` subdirectories if they do not exist.
    pub fn new(root: impl Into<PathBuf>, config: CasStoreConfig) -> miette::Result<Self> {
        let root = root.into();
        let objects_dir = root.join("objects");
        let tmp_dir = root.join("tmp");

        debug!(root = ?root, "Opening CAS store");

        fs::create_dir_all(&objects_dir)?;
        fs::create_dir_all(&tmp_dir)?;

        Ok(Self {
            root,
            objects_dir,
            tmp_dir,
            config,
        })
    }

    /// The root directory of this store.
    pub fn root(&self) -> &Path {
        &self.root
    }

    // ---- Write operations ----

    /// Store raw bytes. Returns the content hash. Idempotent: if the blob
    /// already exists, this is a no-op that returns the hash immediately.
    #[instrument(skip(self, bytes), fields(len = bytes.len()))]
    pub fn write_bytes(&self, bytes: &[u8]) -> miette::Result<ContentHash> {
        let hash = ContentHash::from_blake3(blake3::hash(bytes));

        if self.object_path(&hash).exists() {
            return Ok(hash);
        }

        let mut guard = self.create_temp_file()?;
        {
            let mut file = std::fs::File::create(&guard.path).map_err(|error| {
                CasError::WriteFailed {
                    path: guard.path.clone(),
                    error: Box::new(error),
                }
            })?;
            file.write_all(bytes).map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;
            file.sync_all().map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;
        }

        self.commit_temp(&hash, &mut guard)?;

        debug!(hash = %hash, "Stored blob from bytes");
        Ok(hash)
    }

    /// Store content read from a file at `source`. Uses memory-mapped I/O for
    /// hashing files larger than the configured threshold.
    #[instrument(skip(self))]
    pub fn write_file(&self, source: &Path) -> miette::Result<ContentHash> {
        let hash = self.hash_file(source)?;

        if self.object_path(&hash).exists() {
            return Ok(hash);
        }

        let mut guard = self.create_temp_file()?;
        fs::copy_file(source, &guard.path)?;
        self.sync_file(&guard.path)?;
        self.commit_temp(&hash, &mut guard)?;

        debug!(hash = %hash, source = ?source, "Stored blob from file");
        Ok(hash)
    }

    /// Store content from a streaming reader. Hashes and writes simultaneously
    /// in 64 KiB chunks.
    pub fn write_stream<R: Read>(&self, mut reader: R) -> miette::Result<ContentHash> {
        let mut guard = self.create_temp_file()?;

        let hash = {
            let mut hasher = blake3::Hasher::new();
            let mut file = std::fs::File::create(&guard.path).map_err(|error| {
                CasError::WriteFailed {
                    path: guard.path.clone(),
                    error: Box::new(error),
                }
            })?;

            let mut buf = [0u8; 64 * 1024];
            loop {
                let n = reader.read(&mut buf).map_err(|error| CasError::ReadFailed {
                    path: guard.path.clone(),
                    error: Box::new(error),
                })?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                file.write_all(&buf[..n])
                    .map_err(|error| CasError::WriteFailed {
                        path: guard.path.clone(),
                        error: Box::new(error),
                    })?;
            }

            file.sync_all().map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;

            ContentHash::from_blake3(hasher.finalize())
        };

        if self.object_path(&hash).exists() {
            // Duplicate; guard drops and cleans up the temp file.
            return Ok(hash);
        }

        self.commit_temp(&hash, &mut guard)?;

        debug!(hash = %hash, "Stored blob from stream");
        Ok(hash)
    }

    // ---- Read operations ----

    /// Check whether a blob exists for the given hash.
    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.object_path(hash).exists()
    }

    /// Return the on-disk path to a blob, or `None` if it does not exist.
    pub fn blob_path(&self, hash: &ContentHash) -> Option<PathBuf> {
        let path = self.object_path(hash);
        path.exists().then_some(path)
    }

    /// Read the full blob into memory. Verifies integrity if configured.
    #[instrument(skip(self))]
    pub fn read_bytes(&self, hash: &ContentHash) -> miette::Result<Vec<u8>> {
        let path = self.existing_object_path(hash)?;
        let bytes = fs::read_file_bytes(&path)?;

        if self.config.verify_on_read {
            self.verify_integrity(&path, hash, &bytes)?;
        }

        Ok(bytes)
    }

    /// Open the blob as a [`std::fs::File`] handle for streaming reads.
    /// Verifies integrity before returning the handle if configured.
    pub fn open(&self, hash: &ContentHash) -> miette::Result<std::fs::File> {
        let path = self.existing_object_path(hash)?;

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
        let src = self.existing_object_path(hash)?;

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        match std::fs::hard_link(&src, dest) {
            Ok(()) => {}
            Err(_) => {
                fs::copy_file(&src, dest)?;
            }
        }

        debug!(hash = %hash, dest = ?dest, "Linked blob to destination");
        Ok(())
    }

    /// Ingest a file into the store by hard-linking it. Falls back to
    /// `write_file` if hard-linking fails. Returns the content hash.
    #[instrument(skip(self))]
    pub fn link_from(&self, source: &Path) -> miette::Result<ContentHash> {
        let hash = self.hash_file(source)?;
        let object_path = self.object_path(&hash);

        if object_path.exists() {
            return Ok(hash);
        }

        self.ensure_shard_dir(&hash)?;

        match std::fs::hard_link(source, &object_path) {
            Ok(()) => {
                self.set_readonly(&object_path)?;
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
    pub fn gc(&self, max_age: Duration) -> miette::Result<GcResult> {
        crate::gc::gc(self, max_age)
    }

    /// Remove all blobs from the store.
    pub fn purge(&self) -> miette::Result<GcResult> {
        crate::gc::purge(self)
    }

    /// Update a blob's mtime to now, keeping it alive through GC.
    pub fn touch(&self, hash: &ContentHash) -> miette::Result<()> {
        let path = self.existing_object_path(hash)?;

        // Temporarily make writable so we can update mtime.
        self.set_writable(&path)?;

        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|error| CasError::WriteFailed {
                path: path.clone(),
                error: Box::new(error),
            })?;
        file.set_modified(SystemTime::now())
            .map_err(|error| CasError::WriteFailed {
                path: path.clone(),
                error: Box::new(error),
            })?;
        drop(file);

        // Restore read-only.
        self.set_readonly(&path)?;

        Ok(())
    }

    // ---- Internal helpers ----

    pub(crate) fn objects_dir(&self) -> &Path {
        &self.objects_dir
    }

    pub(crate) fn tmp_dir(&self) -> &Path {
        &self.tmp_dir
    }

    fn object_path(&self, hash: &ContentHash) -> PathBuf {
        self.objects_dir.join(hash.prefix()).join(hash.suffix())
    }

    fn existing_object_path(&self, hash: &ContentHash) -> miette::Result<PathBuf> {
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
        let path = self.tmp_dir.join(Uuid::new_v4().to_string());
        Ok(TempGuard {
            path,
            committed: false,
        })
    }

    fn commit_temp(&self, hash: &ContentHash, guard: &mut TempGuard) -> miette::Result<()> {
        self.ensure_shard_dir(hash)?;
        let dest = self.object_path(hash);
        fs::rename(&guard.path, &dest)?;
        guard.committed = true;
        self.set_readonly(&dest)?;
        Ok(())
    }

    fn sync_file(&self, path: &Path) -> miette::Result<()> {
        let file =
            std::fs::File::open(path).map_err(|error| CasError::WriteFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
        file.sync_all().map_err(|error| CasError::WriteFailed {
            path: path.to_owned(),
            error: Box::new(error),
        })?;
        Ok(())
    }

    fn hash_file(&self, path: &Path) -> miette::Result<ContentHash> {
        let metadata =
            std::fs::metadata(path).map_err(|error| CasError::ReadFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?;

        let mut hasher = blake3::Hasher::new();

        if metadata.len() >= self.config.mmap_threshold {
            // Memory-map large files for fast hashing.
            hasher.update_mmap(path).map_err(|error| CasError::ReadFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
        } else {
            let bytes = fs::read_file_bytes(path)?;
            hasher.update(&bytes);
        }

        Ok(ContentHash::from_blake3(hasher.finalize()))
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

    fn set_writable(&self, path: &Path) -> miette::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))
                .map_err(|error| CasError::WriteFailed {
                    path: path.to_owned(),
                    error: Box::new(error),
                })?;
        }
        #[cfg(not(unix))]
        {
            let mut perms = std::fs::metadata(path)
                .map_err(|error| CasError::ReadFailed {
                    path: path.to_owned(),
                    error: Box::new(error),
                })?
                .permissions();
            perms.set_readonly(false);
            std::fs::set_permissions(path, perms).map_err(|error| CasError::WriteFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
        }
        Ok(())
    }

    fn set_readonly(&self, path: &Path) -> miette::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o444))
                .map_err(|error| CasError::WriteFailed {
                    path: path.to_owned(),
                    error: Box::new(error),
                })?;
        }
        #[cfg(not(unix))]
        {
            let mut perms = std::fs::metadata(path)
                .map_err(|error| CasError::ReadFailed {
                    path: path.to_owned(),
                    error: Box::new(error),
                })?
                .permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(path, perms).map_err(|error| CasError::WriteFailed {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
        }
        Ok(())
    }
}

