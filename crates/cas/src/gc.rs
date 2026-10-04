use crate::cas::CasStore;
use miette::IntoDiagnostic;
use moon_blob::BlobCleanStats;
use moon_hash::ContentHash;
use rustc_hash::FxHashSet;
use starbase_utils::fs;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::task::JoinSet;
use tracing::{debug, instrument};

/// Remove blobs whose mtime is older than `max_age`, and clean orphaned temp files.
#[instrument(skip(store))]
pub async fn gc(store: &CasStore, max_age: Duration) -> miette::Result<BlobCleanStats> {
    let now = SystemTime::now();
    let purge = max_age.is_zero();
    let mut set = JoinSet::new();

    if purge {
        debug!("Purging CAS store");
    } else {
        debug!(?max_age, "Running CAS garbage collection");
    }

    for shard_entry in fs::read_dir(&store.objects_dir)? {
        let shard_path = shard_entry.path();

        if !shard_path.is_dir() {
            continue;
        }

        set.spawn_blocking(move || {
            let mut stats = BlobCleanStats::default();

            for blob_entry in fs::read_dir(&shard_path)? {
                let blob_path = blob_entry.path();
                let metadata = fs::metadata(&blob_path)?;

                let do_remove = if purge {
                    true
                } else if let Ok(modified) = metadata.modified() {
                    now.duration_since(modified).unwrap_or_default() > max_age
                } else {
                    false
                };

                if do_remove {
                    let size = metadata.len();

                    fs::remove_file(&blob_path)?;

                    stats.blobs_removed += 1;
                    stats.bytes_saved += size;
                }
            }

            if purge {
                fs::remove_dir_all(&shard_path)?;
            }

            Ok::<_, miette::Report>(stats)
        });
    }

    let mut stats = BlobCleanStats::default();

    while let Some(result) = set.join_next().await {
        let blob_stats = result.into_diagnostic()??;

        stats.blobs_removed += blob_stats.blobs_removed;
        stats.bytes_saved += blob_stats.bytes_saved;
    }

    if purge {
        // Clean all temp files
        fs::remove_dir_all(&store.temp_dir)?;

        debug!(
            blobs_removed = stats.blobs_removed,
            bytes_saved = stats.bytes_saved,
            "CAS purge complete"
        );
    } else {
        // Clean orphaned temp files (older than 1 hour)
        clean_temp_dir(store, Duration::from_secs(3600))?;

        debug!(
            blobs_removed = stats.blobs_removed,
            bytes_saved = stats.bytes_saved,
            "CAS garbage collection complete"
        );
    }

    Ok(stats)
}

/// Remove all blobs from the store.
pub async fn purge(store: &CasStore) -> miette::Result<BlobCleanStats> {
    gc(store, Duration::ZERO).await
}

/// Reachability sweep: remove objects not present in `keep`, sparing any
/// modified within `grace`. A blob is kept when a surviving manifest still
/// references it; the grace window protects a freshly-written blob whose
/// manifest hasn't landed yet (blobs are stored before the manifest).
pub async fn retain(
    store: &CasStore,
    keep: Arc<FxHashSet<ContentHash>>,
    grace: Duration,
) -> miette::Result<BlobCleanStats> {
    retain_with_released(store, keep, Arc::new(FxHashSet::default()), grace, grace).await
}

/// Reachability sweep that distinguishes why an object is unreferenced:
///
/// - Objects in `keep` are always kept.
/// - Objects in `released` were only referenced by manifests that have been
///   evicted, and are removed once older than `grace`.
/// - Objects that no manifest has ever referenced, like hash manifests that
///   are stored on their own, are removed once older than `unreferenced_grace`
///   (and never sooner than `grace`, as they may be mid-ingest).
#[instrument(skip(store, keep, released))]
pub async fn retain_with_released(
    store: &CasStore,
    keep: Arc<FxHashSet<ContentHash>>,
    released: Arc<FxHashSet<ContentHash>>,
    grace: Duration,
    unreferenced_grace: Duration,
) -> miette::Result<BlobCleanStats> {
    let now = SystemTime::now();
    let unreferenced_grace = unreferenced_grace.max(grace);
    let mut set = JoinSet::new();

    debug!(
        roots = keep.len(),
        released = released.len(),
        ?grace,
        ?unreferenced_grace,
        "Running CAS reachability sweep"
    );

    for shard_entry in fs::read_dir(&store.objects_dir)? {
        let shard_path = shard_entry.path();

        if !shard_path.is_dir() || shard_path == store.temp_dir {
            continue;
        }

        let keep = Arc::clone(&keep);
        let released = Arc::clone(&released);

        set.spawn_blocking(move || {
            let mut stats = BlobCleanStats::default();

            // The shard directory name is the hash prefix; the file name is the
            // suffix. Concatenated, they reconstruct the object's full hash.
            let Some(prefix) = shard_path.file_name().and_then(|name| name.to_str()) else {
                return Ok(stats);
            };

            for blob_entry in fs::read_dir(&shard_path)? {
                let blob_path = blob_entry.path();

                let hash = blob_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .and_then(|suffix| ContentHash::from_hex(format!("{prefix}{suffix}")).ok());

                if hash.as_ref().is_some_and(|hash| keep.contains(hash)) {
                    continue;
                }

                // Objects that were released by an evicted manifest (or aren't
                // valid objects) only get the ingest grace, while objects that
                // were never referenced by a manifest live out the full window
                let window = match &hash {
                    Some(hash) if !released.contains(hash) => unreferenced_grace,
                    _ => grace,
                };

                let metadata = fs::metadata(&blob_path)?;

                // Unreferenced, but spare it if it was written within the
                // window (it may be mid-ingest, manifest not yet stored)
                let within_grace = metadata
                    .modified()
                    .map(|modified| now.duration_since(modified).unwrap_or_default() <= window)
                    .unwrap_or(false);

                if within_grace {
                    continue;
                }

                let size = metadata.len();

                fs::remove_file(&blob_path)?;

                stats.blobs_removed += 1;
                stats.bytes_saved += size;
            }

            Ok::<_, miette::Report>(stats)
        });
    }

    let mut stats = BlobCleanStats::default();

    while let Some(result) = set.join_next().await {
        let blob_stats = result.into_diagnostic()??;

        stats.blobs_removed += blob_stats.blobs_removed;
        stats.bytes_saved += blob_stats.bytes_saved;
    }

    clean_temp_dir(store, Duration::from_secs(3600))?;

    debug!(
        blobs_removed = stats.blobs_removed,
        bytes_saved = stats.bytes_saved,
        "CAS reachability sweep complete"
    );

    Ok(stats)
}

fn clean_temp_dir(store: &CasStore, max_age: Duration) -> miette::Result<()> {
    let now = SystemTime::now();

    if !store.temp_dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&store.temp_dir)? {
        let path = entry.path();
        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(now);

        if now.duration_since(modified).unwrap_or_default() > max_age {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}
