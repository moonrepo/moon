use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_blob::{Blob, BlobCleanStats, BlobContent, BlobInput, BlobOutput};
use moon_cache_storage::{CacheCapabilities, CacheContext, Manifest, StorageBackend};
use moon_cas::CasStore;
use moon_common::Id;
use moon_hash::{ContentHash, Digest};
use rustc_hash::FxHashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, SystemTime};
use tokio::task::spawn_blocking;

/// How long an unreferenced blob is spared from the reachability sweep, covering
/// the window between writing a blob and storing the manifest that references it.
const BLOB_GRACE: Duration = Duration::from_secs(3600);

#[derive(Debug)]
pub struct LocalStorage {
    id: Id,
    #[allow(dead_code)]
    context: CacheContext,

    // States
    capabilities: OnceLock<CacheCapabilities>,

    // Stores
    blobs: Arc<CasStore>,
    manifests: Arc<CasStore>,
}

impl LocalStorage {
    pub fn new(
        context: CacheContext,
        cache_dir: impl AsRef<Path>,
        shared: bool,
    ) -> miette::Result<Self> {
        let cache_dir = cache_dir.as_ref();

        // Support for legacy cache directory structure
        let ac_dir = cache_dir.join("ac");
        let manifests_dir = cache_dir.join("manifests");

        let cas_dir = cache_dir.join("cas");
        let blobs_dir = cache_dir.join("blobs");

        if ac_dir.exists() {
            let _ = fs::rename(ac_dir, &manifests_dir);
        }

        if cas_dir.exists() {
            let _ = fs::rename(cas_dir, &blobs_dir);
        }

        let cas_config = context.cache_config.cas.clone();

        Ok(Self {
            capabilities: OnceLock::new(),
            id: Id::raw(if shared {
                "shared-local-cache"
            } else {
                "local-cache"
            }),
            manifests: Arc::new(CasStore::new(manifests_dir, {
                let mut config = cas_config.clone();
                // Our manifest hashes do not align with their contents,
                // so avoid verifying integrity for now!
                config.verify_integrity = false;
                config
            })?),
            blobs: Arc::new(CasStore::new(blobs_dir, cas_config)?),
            context,
        })
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    fn get_id(&self) -> &Id {
        &self.id
    }

    fn get_capabilities(&self) -> &CacheCapabilities {
        self.capabilities.get_or_init(CacheCapabilities::default)
    }

    fn is_readable(&self) -> bool {
        true
    }

    fn is_writable(&self) -> bool {
        true
    }

    async fn gc(&self, lifetime: Duration) -> miette::Result<BlobCleanStats> {
        let manifests = Arc::clone(&self.manifests);
        let blobs = Arc::clone(&self.blobs);
        let max_size = self
            .context
            .cache_config
            .cas
            .max_size
            .as_deref()
            .and_then(parse_byte_size);

        // 1. Evict stale manifests (the GC roots) and collect the blob digests
        //    the survivors still reference. Runs on a blocking thread: it's all
        //    synchronous filesystem reads and JSON parsing.
        let (keep, removed, saved) =
            spawn_blocking(move || evict_manifests(manifests, lifetime, max_size))
                .await
                .into_diagnostic()??;

        // 2. Sweep blobs no surviving manifest references (past the ingest grace).
        let blob_stats = blobs.retain(Arc::new(keep), BLOB_GRACE).await?;

        Ok(BlobCleanStats {
            blobs_removed: removed + blob_stats.blobs_removed,
            bytes_saved: saved + blob_stats.bytes_saved,
        })
    }

    async fn retrieve_manifest(&self, digest: Digest) -> miette::Result<Option<Manifest>> {
        let manifests = Arc::clone(&self.manifests);

        spawn_blocking(move || {
            if manifests.contains_object(&digest) {
                let blob = manifests.read(&digest)?;
                let manifest: Manifest = serde_json::from_slice(&blob).into_diagnostic()?;

                // Refresh the manifest's mtime so GC treats it as recently used:
                // a hit keeps it (and, by reachability, its blobs) alive, making
                // eviction LRU rather than a fixed TTL. Best-effort.
                let _ = manifests.touch(&digest);

                return Ok(Some(manifest));
            }

            Ok(None)
        })
        .await
        .into_diagnostic()?
    }

    async fn store_manifest(&self, digest: Digest, manifest: Manifest) -> miette::Result<()> {
        let manifests = Arc::clone(&self.manifests);

        spawn_blocking(move || {
            if !manifests.contains_object(&digest) {
                let blob = Blob::from_data(manifest)?;

                manifests.write(&digest, &blob.bytes)?;
            }

            Ok(())
        })
        .await
        .into_diagnostic()?
    }

    async fn find_missing_blobs(
        &self,
        mut blob_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.blobs);

        spawn_blocking(move || {
            blob_digests.retain(|digest| !blobs.contains_object(digest));
            blob_digests
        })
        .await
        .into_diagnostic()
    }

    async fn retrieve_blobs(
        &self,
        blob_digests: Vec<Digest>,
        _stream: bool,
    ) -> miette::Result<Vec<BlobOutput>> {
        let blobs = Arc::clone(&self.blobs);

        spawn_blocking(move || {
            Ok(blob_digests
                .into_iter()
                .filter_map(|digest| {
                    if blobs.contains_object(&digest) {
                        Some(BlobOutput {
                            content: BlobContent::File(blobs.object_path(&digest)),
                            digest,
                        })
                    } else {
                        None
                    }
                })
                .collect())
        })
        .await
        .into_diagnostic()?
    }

    async fn store_blobs(
        &self,
        blob_inputs: Vec<BlobInput>,
        _stream: bool,
    ) -> miette::Result<Vec<Digest>> {
        let blobs = Arc::clone(&self.blobs);

        spawn_blocking(move || {
            let mut digests = vec![];

            for input in blob_inputs {
                let stored = match input.content {
                    BlobContent::File(abs_path) => blobs.write_file(&input.digest, &abs_path)?,
                    BlobContent::Inline(bytes) => blobs.write(&input.digest, &bytes)?,
                };

                if stored {
                    digests.push(input.digest);
                }
            }

            Ok(digests)
        })
        .await
        .into_diagnostic()?
    }
}

/// Evict stale (and, when a budget is set, over-budget) manifests, returning the
/// set of blob hashes the survivors still reference plus `(removed, freed)`.
///
/// Manifests are the GC roots. Walking them newest-first keeps the most recently
/// used entries; with a `max_size` budget we stop retaining once the unique-blob
/// bytes would exceed it. Blob sizes come from the digests themselves, so the
/// budget never has to stat the blob store. An older manifest whose blobs are
/// already retained costs nothing and is kept "for free".
fn evict_manifests(
    manifests: Arc<CasStore>,
    lifetime: Duration,
    max_size: Option<u64>,
) -> miette::Result<(FxHashSet<ContentHash>, usize, u64)> {
    let now = SystemTime::now();

    struct Entry {
        path: PathBuf,
        mtime: SystemTime,
        file_size: u64,
        digests: Vec<Digest>,
    }

    let mut entries = vec![];

    for path in manifests.object_paths()? {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
            continue;
        };

        entries.push(Entry {
            mtime: metadata.modified().unwrap_or(now),
            file_size: metadata.len(),
            digests: manifest.collect_blob_digests(),
            path,
        });
    }

    entries.sort_by_key(|entry| std::cmp::Reverse(entry.mtime));

    let mut keep = FxHashSet::default();
    let mut keep_size: u64 = 0;
    let mut removed = 0;
    let mut saved = 0;

    for entry in entries {
        let age = now.duration_since(entry.mtime).unwrap_or_default();
        let marginal: u64 = entry
            .digests
            .iter()
            .filter(|digest| !keep.contains(&digest.hash))
            .map(|digest| digest.size.max(0) as u64)
            .sum();

        let over_lifetime = age > lifetime;
        let over_budget = max_size.is_some_and(|max| keep_size + marginal > max);

        if over_lifetime || over_budget {
            let _ = fs::remove_file(&entry.path);
            removed += 1;
            saved += entry.file_size;
        } else {
            for digest in entry.digests {
                let size = digest.size.max(0) as u64;

                if keep.insert(digest.hash) {
                    keep_size += size;
                }
            }
        }
    }

    Ok((keep, removed, saved))
}

/// Parse a human-readable byte size such as `"10gb"`, `"512mib"`, or `"2048"`.
/// Decimal units (kb/mb/gb/tb) are powers of 1000; binary units (kib/mib/gib/tib)
/// are powers of 1024. A bare number is bytes. Returns `None` when unparseable.
fn parse_byte_size(input: &str) -> Option<u64> {
    let trimmed = input.trim().to_lowercase();

    if trimmed.is_empty() {
        return None;
    }

    let boundary = trimmed
        .find(|c: char| c.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    let (number, unit) = trimmed.split_at(boundary);

    let value: f64 = number.trim().parse().ok()?;
    let multiplier: f64 = match unit.trim() {
        "" | "b" => 1.0,
        "k" | "kb" => 1_000.0,
        "kib" => 1_024.0,
        "m" | "mb" => 1_000_000.0,
        "mib" => 1_048_576.0,
        "g" | "gb" => 1_000_000_000.0,
        "gib" => 1_073_741_824.0,
        "t" | "tb" => 1_000_000_000_000.0,
        "tib" => 1_099_511_627_776.0,
        _ => return None,
    };

    Some((value * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::parse_byte_size;

    #[test]
    fn parses_bare_bytes_and_decimal_units() {
        assert_eq!(parse_byte_size("2048"), Some(2048));
        assert_eq!(parse_byte_size("10gb"), Some(10_000_000_000));
        assert_eq!(parse_byte_size("512mb"), Some(512_000_000));
        assert_eq!(parse_byte_size("1.5gb"), Some(1_500_000_000));
    }

    #[test]
    fn parses_binary_units_and_tolerates_spacing_case() {
        assert_eq!(parse_byte_size("1gib"), Some(1_073_741_824));
        assert_eq!(parse_byte_size("512 MiB"), Some(536_870_912));
        assert_eq!(parse_byte_size("  10 GB  "), Some(10_000_000_000));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_byte_size(""), None);
        assert_eq!(parse_byte_size("abc"), None);
        assert_eq!(parse_byte_size("10xb"), None);
    }
}
