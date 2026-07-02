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

/// Walk `root` and return it plus every descendant directory that isn't
/// ignored, so each can be watched individually. Ignored subtrees
/// (`node_modules`, `.git`, `.moon/cache`, ...) are never descended into, so
/// we never register a watch for the directories inside them.
fn collect_watch_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Only descend into real directories — not symlinks, to avoid
                // cycles — that aren't ignored.
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) && !is_ignored(&path) {
                    stack.push(path);
                }
            }
        }

        dirs.push(dir);
    }

    dirs
}

fn create_file_event(workspace_root: &Path, path: &Path, kind: EventKind) -> Option<FileEvent> {
    if is_ignored(path) {
        return None;
    }

    let path_relative = match path.relative_to(workspace_root) {
        Ok(path) => path,
        Err(error) => {
            warn!(
                path = ?path,
                "Ignoring file watcher event that cannot be converted to a relative path: {error}"
            );

            return None;
        }
    };

    Some(FileEvent {
        path_original: path.to_owned(),
        path: path_relative,
        kind,
    })
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

    // Watch each non-ignored directory individually rather than recursively
    // from the root, so we don't register an OS watch for every directory
    // inside `node_modules`/`.git` — which exhausts the inotify watch limit on
    // large repos and is slow to set up. New directories are watched as they
    // appear (see the event loop below).
    let watch_dirs = collect_watch_dirs(&workspace_root);

    // The root must be watchable; descendants are best-effort (one may vanish
    // between listing and watching).
    debouncer
        .watch(&workspace_root, RecursiveMode::NonRecursive)
        .map_err(map_notify_error)?;

    for dir in watch_dirs.iter().skip(1) {
        if let Err(error) = debouncer.watch(dir, RecursiveMode::NonRecursive) {
            trace!(dir = ?dir, "Failed to watch directory: {error}");
        }
    }

    debug!(
        path = ?workspace_root,
        dirs = watch_dirs.len(),
        "File watcher started"
    );

    loop {
        tokio::select! {
            Some(result) = bridge_rx.recv() => {
                match result {
                    Ok(events) => {
                        for event in events {
                            // A newly created directory (a new project, a
                            // restored subtree) must be watched too, since we
                            // watch non-recursively. Register it and any
                            // non-ignored descendants created alongside it.
                            if event.kind.is_create() {
                                for path in &event.paths {
                                    if !is_ignored(path) && path.is_dir() {
                                        for dir in collect_watch_dirs(path) {
                                            let _ = debouncer
                                                .watch(&dir, RecursiveMode::NonRecursive);
                                        }
                                    }
                                }
                            }

                            for path in &event.paths {
                                if let Some(file_event) =
                                    create_file_event(&workspace_root, path, event.kind)
                                {
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
pub async fn start_file_listener<T: Clone + Send + 'static>(
    state: T,
    mut watchers: Vec<BoxedFileWatcher<T>>,
    mut event_rx: broadcast::Receiver<FileEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    debug!("File listener started");

    for watcher in watchers.iter_mut() {
        if let Err(error) = watcher.on_init(state.clone()).await {
            error!("System watcher error: {error}");
        }
    }

    loop {
        tokio::select! {
            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        for watcher in watchers.iter_mut() {
                            if let Err(error) = watcher.on_file_event(state.clone(), &event).await {
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
    fn collect_watch_dirs_skips_ignored_subtrees() {
        use starbase_sandbox::create_empty_sandbox;

        let sandbox = create_empty_sandbox();
        let root = sandbox.path();

        for dir in [
            "src/nested",
            ".moon/tasks",
            ".moon/cache/hashes",
            "node_modules/foo",
            ".git/objects",
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
        }

        let dirs = collect_watch_dirs(root);

        // Watched: the root, source dirs, and `.moon` config dirs.
        assert!(dirs.contains(&root.to_path_buf()));
        assert!(dirs.contains(&root.join("src")));
        assert!(dirs.contains(&root.join("src/nested")));
        assert!(dirs.contains(&root.join(".moon")));
        assert!(dirs.contains(&root.join(".moon/tasks")));

        // Never descended into: `node_modules`, `.git`, and `.moon/cache`.
        assert!(!dirs.contains(&root.join("node_modules")));
        assert!(!dirs.contains(&root.join("node_modules/foo")));
        assert!(!dirs.contains(&root.join(".git")));
        assert!(!dirs.contains(&root.join(".moon/cache")));
        assert!(!dirs.contains(&root.join(".moon/cache/hashes")));
    }

    #[test]
    fn test_not_ignored_moon_config() {
        assert!(!is_ignored(&PathBuf::from(
            "/workspace/.moon/workspace.yml"
        )));
    }

    #[test]
    fn creates_workspace_relative_file_event() {
        let event = create_file_event(
            Path::new("/workspace"),
            Path::new("/workspace/src/main.rs"),
            EventKind::Any,
        )
        .unwrap();

        assert_eq!(event.path.as_str(), "src/main.rs");
    }

    #[test]
    #[cfg(unix)]
    fn skips_file_event_with_invalid_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = PathBuf::from(OsStr::from_bytes(b"/workspace/src/\xFF.rs"));

        assert!(create_file_event(Path::new("/workspace"), &path, EventKind::Any,).is_none());
    }
}
