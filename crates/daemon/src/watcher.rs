use crate::daemon_error::DaemonError;
use notify_debouncer_mini::{DebouncedEventKind, new_debouncer, notify::RecursiveMode};
use rustc_hash::FxHashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, trace, warn};

/// Debounce timeout — events within this window are coalesced
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);

/// Directory names that are always ignored by the watcher
static IGNORED_DIRS: LazyLock<FxHashSet<&'static str>> =
    LazyLock::new(|| FxHashSet::from_iter([".git", ".svn", "node_modules"]));

/// Path segments (multi-component) that are ignored
static IGNORED_PATHS: LazyLock<Vec<[&'static str; 2]>> =
    LazyLock::new(|| vec![[".moon", "cache"], [".moon", "docker"]]);

#[derive(Clone, Debug)]
pub struct FileEvent {
    pub path: PathBuf,
    pub kind: FileEventKind,
}

#[derive(Clone, Debug)]
pub enum FileEventKind {
    /// File or directory was created or modified
    Any,
    /// Continuous/ongoing modification (e.g. a long write)
    AnyContinuous,
}

/// Returns `true` if the path should be ignored by the watcher
fn is_ignored(path: &Path) -> bool {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    // Check single-component ignores
    for &dir in IGNORED_DIRS.iter() {
        if components.contains(&dir) {
            return true;
        }
    }

    // Check multi-component path ignores
    for window in components.windows(2) {
        for ignored in IGNORED_PATHS.iter() {
            if window[0] == ignored[0] && window[1] == ignored[1] {
                return true;
            }
        }
    }

    false
}

fn map_notify_error(error: notify_debouncer_mini::notify::Error) -> DaemonError {
    DaemonError::WatcherFailed {
        error: Box::new(error),
    }
}

/// Start watching the workspace root for file changes.
///
/// File events are debounced and broadcast on `event_tx`. The watcher
/// runs until `shutdown_rx` receives a message, at which point it
/// drops the underlying OS watcher and returns.
///
/// Errors from the `notify` backend are logged but do not stop the
/// watcher — only a shutdown signal does.
pub async fn start_file_watcher(
    workspace_root: PathBuf,
    event_tx: broadcast::Sender<FileEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> miette::Result<()> {
    let (bridge_tx, mut bridge_rx) = mpsc::channel(512);

    // This closure runs on notify's internal thread
    let mut debouncer = new_debouncer(DEBOUNCE_TIMEOUT, move |result| {
        if bridge_tx.blocking_send(result).is_err() {
            // Receiver dropped — watcher is shutting down
        }
    })
    .map_err(map_notify_error)?;

    debouncer
        .watcher()
        .watch(&workspace_root, RecursiveMode::Recursive)
        .map_err(map_notify_error)?;

    debug!(path = ?workspace_root, "File watcher started");

    loop {
        tokio::select! {
            Some(result) = bridge_rx.recv() => {
                match result {
                    Ok(events) => {
                        for event in events {
                            if is_ignored(&event.path) {
                                continue;
                            }

                            let kind = match event.kind {
                                DebouncedEventKind::Any => FileEventKind::Any,
                                DebouncedEventKind::AnyContinuous => FileEventKind::AnyContinuous,
                                _ => FileEventKind::Any,
                            };

                            trace!(path = ?event.path, ?kind, "File event");

                            // Ignore send failures
                            let _ = event_tx.send(FileEvent {
                                path: event.path,
                                kind,
                            });
                        }
                    }
                    Err(error) => {
                        warn!("File watcher error: {error}");
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!("File watcher shutting down");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ignored_git() {
        assert!(is_ignored(&PathBuf::from("/workspace/.git/objects/abc")));
    }

    #[test]
    fn test_is_ignored_node_modules() {
        assert!(is_ignored(&PathBuf::from(
            "/workspace/node_modules/foo/bar.js"
        )));
    }

    #[test]
    fn test_is_ignored_svn() {
        assert!(is_ignored(&PathBuf::from("/workspace/.svn/entries")));
    }

    #[test]
    fn test_is_ignored_moon_cache() {
        assert!(is_ignored(&PathBuf::from(
            "/workspace/.moon/cache/hashes/abc"
        )));
    }

    #[test]
    fn test_is_ignored_moon_docker() {
        assert!(is_ignored(&PathBuf::from(
            "/workspace/.moon/docker/scaffold"
        )));
    }

    #[test]
    fn test_not_ignored_source_file() {
        assert!(!is_ignored(&PathBuf::from("/workspace/src/main.rs")));
    }

    #[test]
    fn test_not_ignored_moon_config() {
        assert!(!is_ignored(&PathBuf::from(
            "/workspace/.moon/workspace.yml"
        )));
    }
}
