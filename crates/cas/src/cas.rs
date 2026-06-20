use crate::cas_error::CasError;
use crate::gc::GcResult;
use moon_blob::Blob;
use moon_config::CacheCasConfig;
use moon_hash::{ContentHash, Digest};
use starbase_utils::fs;
use starbase_utils::hash::{
    self, hex,
    sha256::native::{Digest as ShaDigest, Sha256},
};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument, trace};

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
    pub fn new(root: impl AsRef<Path>, config: &CacheCasConfig) -> miette::Result<Self> {
        let root = root.as_ref();
        let temp_dir = root.join("temp");

        debug!(root = ?root, "Creating CAS store");

        fs::create_dir_all(&temp_dir)?;

        Ok(Self {
            objects_dir: root.to_path_buf(),
            temp_dir,
            config: config.to_owned(),
        })
    }

    // ---- Write operations ----

    #[instrument(skip(self, bytes), fields(len = bytes.len()))]
    pub fn write(&self, hash: &ContentHash, bytes: &[u8]) -> miette::Result<bool> {
        if self.contains_object(hash) {
            return Ok(false);
        }

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

    /// Store raw bytes from the provided blob.
    pub fn write_blob(&self, blob: &Blob) -> miette::Result<()> {
        if self.write(&blob.digest.hash, &blob.bytes)? {
            trace!(hash = blob.digest.hash.as_str(), "Stored object from blob");
        }

        Ok(())
    }

    /// Store raw bytes and return the content hash.
    pub fn write_bytes(&self, bytes: &[u8]) -> miette::Result<Digest> {
        let digest = Digest::from_bytes(bytes)?;

        if self.write(&digest.hash, bytes)? {
            trace!(hash = digest.hash.as_str(), "Stored object from bytes");
        }

        Ok(digest)
    }

    /// Store content read from a file and return a blob.
    #[instrument(skip(self))]
    pub fn write_file(&self, path: &Path) -> miette::Result<Blob> {
        let blob = Blob::from_file(path)?;

        if self.write(&blob.digest.hash, &blob.bytes)? {
            trace!(hash = blob.digest.hash.as_str(), path = ?path, "Stored object from file");
        }

        Ok(blob)
    }

    /// Store content from a file by streaming it through the store.
    ///
    /// Unlike [`Self::write_stream`], the file is hashed up front in a single
    /// read-only pass (in 64 KiB chunks, never materialized in memory), so an
    /// object that already exists short-circuits *before* any temp file is
    /// created. Only a genuine cache miss copies the bytes into the store.
    ///
    /// Prefer this over `write_stream` whenever the source is a real file: it
    /// avoids the wasteful temp-file create/write/remove churn that streaming
    /// incurs on a warm cache, where the hash isn't known until after the
    /// throwaway temp has already been written.
    #[instrument(skip(self))]
    pub fn write_path(&self, path: &Path) -> miette::Result<Digest> {
        // Content is addressed by hash, so the file must be read to know
        // whether the store already holds it. Hash without writing anything.
        let digest = Digest::from_file(path)?;

        // Warm cache: object already present, so skip all temp-file work.
        if self.contains_object(&digest.hash) {
            return Ok(digest);
        }

        // Cold cache: copy the file into a temp file, then atomically commit.
        // Outputs are stable at archive time, so re-reading here is safe and
        // the bytes are almost certainly still warm in the OS page cache.
        let mut guard = self.create_temp_file()?;

        {
            let mut source = fs::open_file(path)?;
            let mut file = fs::create_file(&guard.path)?;

            std::io::copy(&mut source, &mut file).map_err(|error| CasError::WriteFailed {
                path: guard.path.clone(),
                error: Box::new(error),
            })?;
        }

        // No fsync: see `write` for rationale.
        self.commit_temp_file(&digest.hash, &mut guard)?;

        trace!(hash = digest.hash.as_str(), path = ?path, "Stored object from file stream");

        Ok(digest)
    }

    /// Store content from a streaming reader.
    /// Hashes and writes simultaneously in 64 KiB chunks.
    ///
    /// The hash of a stream isn't known until it is fully consumed, so this
    /// always writes a temp file and only then checks for an existing object —
    /// discarding the temp on a hit. When the source is a file, prefer
    /// [`Self::write_path`], which avoids that churn.
    pub fn write_stream<R: Read>(&self, mut reader: R) -> miette::Result<Digest> {
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
    /// happens lazily on read (via `read_bytes` / `open`). Putting it here
    /// would force a full file read + rehash on every write to a hash that
    /// already exists, which dominates the cost of a warm cache.
    pub fn contains_object(&self, hash: &ContentHash) -> bool {
        self.object_path(hash).exists()
    }

    /// Read the full blob into memory. Verifies integrity if configured.
    #[instrument(skip(self))]
    pub fn read_bytes(&self, hash: &ContentHash) -> miette::Result<Vec<u8>> {
        let path = self.object_path_with_exists_check(hash)?;
        let bytes = fs::read_file_bytes(&path)?;

        if self.config.verify_integrity {
            self.verify_integrity(&path, hash, &bytes)?;
        }

        Ok(bytes)
    }

    /// Open the blob as a [`std::fs::File`] handle for streaming reads.
    /// Verifies integrity before returning the handle if configured.
    pub fn open(&self, hash: &ContentHash) -> miette::Result<std::fs::File> {
        let path = self.object_path_with_exists_check(hash)?;

        if self.config.verify_integrity {
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

    // ---- Lifecycle ----

    /// Remove blobs whose mtime is older than `max_age`.
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
        let file = fs::open_file_for_writing(&path)?;

        file.set_modified(SystemTime::now())
            .map_err(|error| CasError::WriteFailed {
                path: path.clone(),
                error: Box::new(error),
            })?;

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

        fs::rename(&guard.path, &dest)?;

        guard.committed = true;

        Ok(())
    }

    fn verify_integrity(
        &self,
        path: &Path,
        expected: &ContentHash,
        bytes: &[u8],
    ) -> miette::Result<()> {
        let actual = hash::sha256::from_bytes(bytes);

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
