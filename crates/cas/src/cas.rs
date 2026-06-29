use crate::cas_error::CasError;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobCleanStats};
use moon_config::CacheCasConfig;
use moon_hash::{ContentHash, Digest};
use rustc_hash::FxHashSet;
use starbase_utils::fs;
use starbase_utils::hash::{
    self, hex,
    sha256::native::{Digest as ShaDigest, Sha256},
};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument, trace};

// NOTE: We avoid using `starbase_utils::fs` for some operations as they
// spam the logs with far too much useless information!

/// A content-addressable file system store.
///
/// Content is addressed by its SHA256 hash and stored in a git-style
/// prefix-sharded directory layout. Writes are atomic (temp file + rename)
/// so readers never observe partial data, and no file locking is required.
#[derive(Debug)]
pub struct CasStore {
    pub objects_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub config: CacheCasConfig,
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
    pub fn new(root: impl AsRef<Path>, config: CacheCasConfig) -> miette::Result<Self> {
        let root = root.as_ref();
        let temp_dir = root.join("temp");

        debug!(root = ?root, "Creating CAS store");

        fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            objects_dir: root.to_path_buf(),
            temp_dir,
            config,
        })
    }

    // ---- Write operations ----

    #[instrument(skip(self, bytes), fields(size = bytes.len()))]
    pub fn write(&self, hash: &ContentHash, bytes: &[u8]) -> miette::Result<bool> {
        let mut guard = self.create_temp_file()?;

        {
            let mut file = fs::create_file(&guard.path)?;

            file.write_all(bytes)
                .map_err(|error| CasError::WriteFailed {
                    path: guard.path.clone(),
                    error: Box::new(error),
                })?;
        }

        // No fsync: the temp-file + rename pattern guarantees readers never
        // see partial content. Power-loss before durability flush would leave
        // a missing blob, which on the next run just becomes a cache miss.
        self.commit_temp_file(hash, &mut guard)?;

        Ok(true)
    }

    #[instrument(skip(self))]
    pub fn write_file(&self, hash: &ContentHash, source: &Path) -> miette::Result<bool> {
        // Cold cache: reflink (copy-on-write clone) the file into a temp file,
        // then atomically commit. On a reflink-capable filesystem this shares
        // blocks instead of copying bytes, so ingesting a fresh output is
        // near-instant and costs no extra disk space; `reflink_file` falls back
        // to a plain copy otherwise. We still stage through a temp + rename so
        // readers never observe a partial object and the non-atomic copy
        // fallback stays safe.
        let mut guard = self.create_temp_file()?;

        fs::reflink_file(source, &guard.path)?;

        // No fsync: see `write` for rationale.
        self.commit_temp_file(hash, &mut guard)?;

        Ok(true)
    }

    /// Store raw bytes from the provided blob.
    pub fn store_blob(&self, blob: &Blob) -> miette::Result<()> {
        if !self.contains_object(&blob.digest) && self.write(&blob.digest, &blob.bytes)? {
            trace!(hash = blob.digest.hash.as_str(), "Stored object from blob");
        }

        Ok(())
    }

    /// Store raw bytes and return the associated digest.
    pub fn store_bytes(&self, bytes: &[u8]) -> miette::Result<Digest> {
        let digest = Digest::from_bytes(bytes)?;

        if !self.contains_object(&digest) && self.write(&digest, bytes)? {
            trace!(hash = digest.hash.as_str(), "Stored object from bytes");
        }

        Ok(digest)
    }

    /// Store the contents of a file and return the associated digest.
    /// Internally this will attempt to reflink (copy-on-write clone) the file into
    /// the store, falling back to a plain copy if the filesystem doesn't support it.
    pub fn store_file(&self, path: &Path) -> miette::Result<Digest> {
        let digest = Digest::from_file(path)?;

        if !self.contains_object(&digest) && self.write_file(&digest, path)? {
            trace!(hash = digest.hash.as_str(), path = ?path, "Stored object from file");
        }

        Ok(digest)
    }

    /// Store content from a streaming reader.
    /// Hashes and writes simultaneously in 64 KiB chunks.
    ///
    /// The hash of a stream isn't known until it is fully consumed, so this
    /// always writes a temp file and only then checks for an existing object —
    /// discarding the temp on a hit. When the source is a file, prefer
    /// [`Self::store_file`], which avoids that churn.
    pub fn store_stream<R: Read>(&self, mut reader: R) -> miette::Result<Digest> {
        let mut guard = self.create_temp_file()?;
        let mut size = 0;

        let hash = {
            let mut hasher = Sha256::default();
            let mut file = fs::create_file(&guard.path)?;
            let mut buffer = [0u8; 64 * 1024];

            loop {
                let n = reader
                    .read(&mut buffer)
                    .map_err(|error| CasError::ReadFailed {
                        path: guard.path.clone(),
                        error: Box::new(error),
                    })?;

                size += n;

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

            // No fsync: see `write` for rationale.
            ContentHash::from_hex(hex::encode(hasher.finalize()))?
        };

        let digest = Digest {
            hash,
            size: size as i64,
        };

        if self.contains_object(&digest.hash) {
            return Ok(digest);
        }

        self.commit_temp_file(&digest.hash, &mut guard)?;

        trace!(
            hash = digest.hash.as_str(),
            "Stored object from byte stream"
        );

        Ok(digest)
    }

    // ---- Read operations ----

    /// Check whether an object exists for the given hash.
    ///
    /// This is a pure existence check; it does not verify the on-disk content
    /// against the hash even when `verify_integrity` is enabled. Verification
    /// happens lazily on read (via `read`, etc). Putting it here
    /// would force a full file read + rehash on every write to a hash that
    /// already exists, which dominates the cost of a warm cache.
    pub fn contains_object(&self, hash: &ContentHash) -> bool {
        self.object_path(hash).exists()
    }

    /// Read the full blob into memory. Verifies integrity if configured.
    #[instrument(skip(self))]
    pub fn read(&self, hash: &ContentHash) -> miette::Result<Vec<u8>> {
        let path = self.object_path_with_exists_check(hash)?;
        let bytes = fs::read_file_bytes(&path)?;

        if self.config.verify_integrity {
            self.verify_integrity(&path, hash, hash::sha256::from_bytes(&bytes))?;
        }

        Ok(bytes)
    }

    /// Read the object from the cache and write it to the destination path.
    /// Verifies integrity if configured.
    ///
    /// Uses a reflink (copy-on-write clone) so the destination shares
    /// storage with the stored object: near-instant and zero extra disk space
    /// on a reflink-capable filesystem, falling back to a plain copy otherwise.
    #[instrument(skip(self))]
    pub fn read_file(&self, hash: &ContentHash, dest: &Path) -> miette::Result<()> {
        let path = self.object_path_with_exists_check(hash)?;

        if self.config.verify_integrity {
            self.verify_integrity(&path, hash, hash::sha256::from_file(&path)?)?;
        }

        // A reflink only takes the fast clone path when the destination doesn't
        // exist; clear any stale file so we never silently fall back to a copy.
        if dest.symlink_metadata().is_ok() {
            fs::remove_file(dest)?;
        }

        fs::reflink_file(&path, dest)?;

        Ok(())
    }

    /// Retrieve the bytes of an object by its hash and return a blob.
    pub fn retrieve_blob(&self, hash: &ContentHash) -> miette::Result<Blob> {
        let bytes = self.read(hash)?;

        Ok(Blob::new(
            Digest {
                hash: hash.clone(),
                size: bytes.len() as i64,
            },
            bytes,
        ))
    }

    /// Retrieve the bytes of an object by its hash.
    pub fn retrieve_bytes(&self, hash: &ContentHash) -> miette::Result<Vec<u8>> {
        self.read(hash)
    }

    // ---- Lifecycle ----

    /// Remove blobs whose mtime is older than `max_age`.
    pub async fn gc(&self, max_age: Duration) -> miette::Result<BlobCleanStats> {
        crate::gc::gc(self, max_age).await
    }

    /// Remove all blobs from the store.
    pub async fn purge(&self) -> miette::Result<BlobCleanStats> {
        crate::gc::purge(self).await
    }

    /// Reachability sweep: remove every object whose hash is not in `keep`,
    /// except objects modified within `grace` (which protects a blob written
    /// just before the manifest that references it, mid-ingest).
    pub async fn retain(
        &self,
        keep: Arc<FxHashSet<ContentHash>>,
        grace: Duration,
    ) -> miette::Result<BlobCleanStats> {
        crate::gc::retain(self, keep, grace).await
    }

    /// Update a blob's mtime to now, keeping it alive through GC.
    pub fn touch(&self, hash: &ContentHash) -> miette::Result<()> {
        let path = self.object_path_with_exists_check(hash)?;
        let file = fs::open_file_for_writing(&path)?;

        file.set_modified(SystemTime::now())
            .map_err(|error| CasError::WriteFailed {
                path: path.clone(),
                error: Box::new(error),
            })?;

        Ok(())
    }

    // ---- Helpers ----

    pub fn object_path(&self, hash: &ContentHash) -> PathBuf {
        self.objects_dir.join(hash.prefix()).join(hash.suffix())
    }

    /// Paths of every stored object, excluding the temp/staging directory.
    pub fn object_paths(&self) -> miette::Result<Vec<PathBuf>> {
        let mut paths = vec![];

        for shard in fs::read_dir(&self.objects_dir)? {
            let shard_path = shard.path();

            if !shard_path.is_dir() || shard_path == self.temp_dir {
                continue;
            }

            for entry in fs::read_dir(&shard_path)? {
                paths.push(entry.path());
            }
        }

        Ok(paths)
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

    // ---- Internal helpers ----

    fn create_temp_file(&self) -> miette::Result<TempGuard> {
        let key: String = std::iter::repeat_with(fastrand::alphanumeric)
            .take(32)
            .collect();

        Ok(TempGuard {
            path: self.temp_dir.join(key),
            committed: false,
        })
    }

    fn commit_temp_file(&self, hash: &ContentHash, guard: &mut TempGuard) -> miette::Result<()> {
        let dest = self.object_path(hash);

        if let Some(shard) = dest.parent() {
            std::fs::create_dir_all(shard).into_diagnostic()?;
        }

        std::fs::rename(&guard.path, &dest).into_diagnostic()?;

        guard.committed = true;

        Ok(())
    }

    fn verify_integrity(
        &self,
        path: &Path,
        hash: &ContentHash,
        actual: String,
    ) -> miette::Result<()> {
        let expected = hash.as_hex();

        if actual != expected {
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
