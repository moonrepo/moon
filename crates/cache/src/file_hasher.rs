use moon_async_utils::run_pooled_blocking_tasks;
use moon_common::path::WorkspaceRelativePathBuf;
use starbase_utils::fs::FsError;
use starbase_utils::hash;
use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::{debug, trace};

/// A file modified this close to when it was hashed may be modified again
/// without its modification time changing, on file systems with coarse
/// timestamps (FAT, HFS+, ext3), so its hash is never memoized. This is
/// the same "racily clean" rule that Git applies to its index.
const RACY_WINDOW: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct MemoEntry {
    len: u64,
    modified: Option<SystemTime>,
    hash: String,
}

/// Hashes file contents on the blocking thread pool, and memoizes the
/// hashes for the lifetime of the process. A memoized hash is only reused
/// while the file's size and modification time are unchanged, so tasks
/// that share inputs (the same project's `build`, `test`, and `lint`, or
/// workspace-wide files like lockfiles) only read each file once.
#[derive(Default)]
pub struct FileHasher {
    memo: Arc<scc::HashMap<PathBuf, MemoEntry>>,
}

impl FileHasher {
    pub async fn hash_files(
        &self,
        root: &Path,
        files: &[WorkspaceRelativePathBuf],
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        debug!("Hashing {} files", files.len());

        let memo = Arc::clone(&self.memo);
        let mut map = BTreeMap::new();

        run_pooled_blocking_tasks(
            files
                .iter()
                .map(|file| (file.clone(), file.to_logical_path(root)))
                .collect(),
            move |(rel_file, abs_file)| {
                Ok(hash_file(abs_file, &memo)?.map(|hash| (rel_file.clone(), hash)))
            },
            |hashed| {
                if let Some((rel_file, hash)) = hashed {
                    map.insert(rel_file, hash);
                }

                Ok(())
            },
        )
        .await?;

        Ok(map)
    }

    /// The number of files with a memoized hash.
    pub fn memoized_count(&self) -> usize {
        self.memo.len()
    }
}

impl fmt::Debug for FileHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileHasher")
            .field("memoized", &self.memo.len())
            .finish()
    }
}

fn hash_file(
    path: &Path,
    memo: &scc::HashMap<PathBuf, MemoEntry>,
) -> miette::Result<Option<String>> {
    // Stat the path before opening it, so that a memoized file costs a
    // single syscall. Missing paths and directories aren't hashed, but
    // are not an error either
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(None);
    };

    if !metadata.is_file() {
        return Ok(None);
    }

    let len = metadata.len();
    let modified = metadata.modified().ok();

    let memoized = memo
        .read_sync(path, |_, entry| {
            (entry.len == len && entry.modified == modified).then(|| entry.hash.clone())
        })
        .flatten();

    if let Some(hash) = memoized {
        trace!(path = ?path, hash, "Reusing memoized file hash");

        return Ok(Some(hash));
    }

    let file = File::open(path).map_err(|error| FsError::Read {
        path: path.to_owned(),
        error: Box::new(error),
    })?;

    let hashed_at = SystemTime::now();
    let hash = hash::sha256::from_reader(file)?;

    let racy = modified.is_none_or(|modified| {
        hashed_at
            .duration_since(modified)
            .is_ok_and(|age| age < RACY_WINDOW)
    });

    if !racy {
        memo.upsert_sync(
            path.to_owned(),
            MemoEntry {
                len,
                modified,
                hash: hash.clone(),
            },
        );
    }

    Ok(Some(hash))
}
