use crate::daemon_server_error::DaemonServerError;
use moon_common::path::PathExt;
use moon_file_watcher::*;
use notify_debouncer_full::{new_debouncer, notify::RecursiveMode};
use rustc_hash::FxHashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, trace, warn};

/// Debounce timeout — events within this window are coalesced
const DEBOUNCE_TIMEOUT: Duration = Duration::from_millis(500);

/// Directory names that are always ignored by the watcher
static IGNORED_DIRS: LazyLock<FxHashSet<&'static str>> =
    LazyLock::new(|| FxHashSet::from_iter([".git", ".svn", "node_modules"]));

/// Path segments (multi-component) that are ignored
static IGNORED_PATHS: LazyLock<Vec<[&'static str; 2]>> =
    LazyLock::new(|| vec![[".moon", "cache"], [".moon", "docker"]]);

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

fn map_notify_error(error: notify_debouncer_full::notify::Error) -> DaemonServerError {
    DaemonServerError::WatcherFailed {
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
    let mut debouncer = new_debouncer(DEBOUNCE_TIMEOUT, None, move |result| {
        if bridge_tx.blocking_send(result).is_err() {
            // Receiver dropped — watcher is shutting down
        }
    })
    .map_err(map_notify_error)?;

    debouncer
        .watch(&workspace_root, RecursiveMode::Recursive)
        .map_err(map_notify_error)?;

    debug!(path = ?workspace_root, "File watcher started");

    loop {
        tokio::select! {
            Some(result) = bridge_rx.recv() => {
                match result {
                    Ok(events) => {
                        for event in events {
                            for path in &event.paths {
                                if is_ignored(path) {
                                    continue;
                                }

                                let file_event = FileEvent {
                                    path_original: path.clone(),
                                    path: path.relative_to(&workspace_root).unwrap(),
                                    kind: event.kind,
                                };

                                // We only care about mutations, not access, etc
                                if file_event.is_mutated() {
                                    trace!(
                                        path = ?file_event.path,
                                        kind = ?file_event.kind,
                                        "File change event",
                                    );

                                    // Ignore send failures
                                    let _ = event_tx.send(file_event);
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            warn!("File watcher error: {error}");
                        }
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

/// Start a file listener that receives file events from `event_rx` and
/// dispatches them to the provided `watchers`. The listener runs until
/// `shutdown_rx` receives a message, at which point it returns.
///
/// Errors from the watchers are logged but do not stop the listener —
/// only a shutdown signal does. Watchers are expected to handle their own internal
/// state and debounce as needed, since file events can arrive in bursts.
pub async fn start_file_listener<T: Send + 'static>(
    mut state: T,
    mut watchers: Vec<BoxedFileWatcher<T>>,
    mut event_rx: broadcast::Receiver<FileEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    debug!("File listener started");

    loop {
        tokio::select! {
            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        for watcher in watchers.iter_mut() {
                            if let Err(error) = watcher.on_file_event(&mut state, &event).await {
                                error!("System watcher error: {error}");
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(count)) => {
                        warn!("File change event receiver lagged by {count} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        debug!("File listener shutting down");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!("File listener shutting down");
                break;
            }
        }
    }
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
