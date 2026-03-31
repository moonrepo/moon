use crate::store::CasStore;
use starbase_utils::fs;
use std::time::{Duration, SystemTime};
use tracing::{debug, instrument};

/// Result of a garbage collection or purge operation.
#[derive(Debug, Default)]
pub struct GcResult {
    pub blobs_removed: usize,
    pub bytes_freed: u64,
}

/// Remove blobs whose mtime is older than `max_age`, and clean orphaned temp files.
#[instrument(skip(store))]
pub(crate) fn gc(store: &CasStore, max_age: Duration) -> miette::Result<GcResult> {
    let now = SystemTime::now();
    let mut result = GcResult::default();

    debug!(?max_age, "Running CAS garbage collection");

    for shard_entry in fs::read_dir(store.objects_dir())? {
        let shard_path = shard_entry.path();
        if !shard_path.is_dir() {
            continue;
        }

        for blob_entry in fs::read_dir(&shard_path)? {
            let blob_path = blob_entry.path();
            let metadata = match std::fs::metadata(&blob_path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let modified = match metadata.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };

            if now.duration_since(modified).unwrap_or_default() > max_age {
                let size = metadata.len();
                // Blobs are read-only; make writable before removal.
                make_writable(&blob_path);
                fs::remove_file(&blob_path)?;
                result.blobs_removed += 1;
                result.bytes_freed += size;
            }
        }

        // Remove empty shard directories to keep the tree clean.
        if is_dir_empty(&shard_path) {
            let _ = std::fs::remove_dir(&shard_path);
        }
    }

    // Clean orphaned temp files from crashed writes (older than 1 hour).
    clean_tmp_dir(store, Duration::from_secs(3600))?;

    debug!(
        blobs_removed = result.blobs_removed,
        bytes_freed = result.bytes_freed,
        "CAS garbage collection complete"
    );

    Ok(result)
}

/// Remove all blobs from the store.
pub(crate) fn purge(store: &CasStore) -> miette::Result<GcResult> {
    let mut result = GcResult::default();

    debug!("Purging CAS store");

    for shard_entry in fs::read_dir(store.objects_dir())? {
        let shard_path = shard_entry.path();
        if !shard_path.is_dir() {
            continue;
        }

        for blob_entry in fs::read_dir(&shard_path)? {
            let blob_path = blob_entry.path();
            let size = std::fs::metadata(&blob_path)
                .map(|m| m.len())
                .unwrap_or(0);
            make_writable(&blob_path);
            fs::remove_file(&blob_path)?;
            result.blobs_removed += 1;
            result.bytes_freed += size;
        }

        let _ = std::fs::remove_dir(&shard_path);
    }

    // Also clean all temp files.
    for entry in fs::read_dir(store.tmp_dir())? {
        let path = entry.path();
        let _ = std::fs::remove_file(&path);
    }

    debug!(
        blobs_removed = result.blobs_removed,
        bytes_freed = result.bytes_freed,
        "CAS purge complete"
    );

    Ok(result)
}

fn clean_tmp_dir(store: &CasStore, max_age: Duration) -> miette::Result<()> {
    let now = SystemTime::now();

    if !store.tmp_dir().exists() {
        return Ok(());
    }

    for entry in fs::read_dir(store.tmp_dir())? {
        let path = entry.path();
        let modified = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .unwrap_or(now);

        if now.duration_since(modified).unwrap_or_default() > max_age {
            let _ = std::fs::remove_file(&path);
        }
    }

    Ok(())
}

fn is_dir_empty(path: &std::path::Path) -> bool {
    std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true)
}

fn make_writable(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644));
    }
    #[cfg(not(unix))]
    {
        if let Ok(metadata) = std::fs::metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
}
