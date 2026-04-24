use crate::cas::CasStore;
use miette::IntoDiagnostic;
use starbase_utils::fs;
use std::time::{Duration, SystemTime};
use tokio::task::JoinSet;
use tracing::{debug, instrument};

/// Result of a garbage collection or purge operation.
#[derive(Debug, Default)]
pub struct GcResult {
    pub blobs_removed: usize,
    pub bytes_freed: u64,
}

/// Remove blobs whose mtime is older than `max_age`, and clean orphaned temp files.
#[instrument(skip(store))]
pub async fn gc(store: &CasStore, max_age: Duration) -> miette::Result<GcResult> {
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
            let mut stats = GcResult::default();

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
                    stats.bytes_freed += size;
                }
            }

            if purge {
                fs::remove_dir_all(&shard_path)?;
            }

            Ok::<_, miette::Report>(stats)
        });
    }

    let mut stats = GcResult::default();

    while let Some(result) = set.join_next().await {
        let blob_stats = result.into_diagnostic()??;

        stats.blobs_removed += blob_stats.blobs_removed;
        stats.bytes_freed += blob_stats.bytes_freed;
    }

    if purge {
        // Clean all temp files
        fs::remove_dir_all(&store.temp_dir)?;

        debug!(
            blobs_removed = stats.blobs_removed,
            bytes_freed = stats.bytes_freed,
            "CAS purge complete"
        );
    } else {
        // Clean orphaned temp files (older than 1 hour)
        clean_temp_dir(store, Duration::from_secs(3600))?;

        debug!(
            blobs_removed = stats.blobs_removed,
            bytes_freed = stats.bytes_freed,
            "CAS garbage collection complete"
        );
    }

    Ok(stats)
}

/// Remove all blobs from the store.
pub async fn purge(store: &CasStore) -> miette::Result<GcResult> {
    gc(store, Duration::ZERO).await
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
