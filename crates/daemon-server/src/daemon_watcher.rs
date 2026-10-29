use crate::daemon_server_error::DaemonServerError;
use moon_common::format_error_chain;
use moon_common::path::PathExt;
use moon_file_watcher::*;
use moon_vcs::BoxedVcs;
use notify_debouncer_full::notify::{
    self, Config, ErrorKind, RecommendedWatcher, RecursiveMode, WatcherKind,
};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, NoCache, new_debouncer_opt,
};
use rustc_hash::FxHashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, mpsc};
use std::thread;
use std::time::Duration;
use tokio::sync::broadcast;
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

fn create_file_event(workspace_root: &Path, path: &Path, kind: EventKind) -> Option<FileEvent> {
    if is_ignored(path) {
        return None;
    }

    let path_relative = match path.relative_to(workspace_root) {
        Ok(path) => path,
        Err(error) => {
            warn!(
                path = ?path,
                error = error.to_string(),
                "Ignoring file watcher event that cannot be converted to a relative path"
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

/// Messages received by the watcher thread.
enum WatcherMessage {
    Events(DebounceEventResult),
    Stop,
}

/// Stops the watcher thread when dropped, so that it also stops when the
/// watcher task is aborted, instead of receiving the shutdown signal.
struct StopWatcherOnDrop {
    stop: Arc<AtomicBool>,
    tx: mpsc::Sender<WatcherMessage>,
}

impl Drop for StopWatcherOnDrop {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        // Receiver dropped — thread has already stopped
        let _ = self.tx.send(WatcherMessage::Stop);
    }
}

/// Owns the OS watcher, registers its watches, and broadcasts file events.
struct WatcherThread {
    debouncer: Debouncer<RecommendedWatcher, NoCache>,
    event_tx: broadcast::Sender<FileEvent>,
    stop: Arc<AtomicBool>,
    vcs: Arc<BoxedVcs>,
    workspace_root: PathBuf,

    /// Whether the backend watches a tree with a single OS-level watch
    /// (an FSEvents stream on macOS, a directory handle on Windows),
    /// instead of a watch per directory (inotify on Linux).
    watches_recursively: bool,
}

impl WatcherThread {
    fn run(mut self, rx: mpsc::Receiver<WatcherMessage>) {
        if self.watches_recursively {
            // Events in ignored directories are filtered by `create_file_event`
            if let Err(error) = self
                .debouncer
                .watch(&self.workspace_root, RecursiveMode::Recursive)
            {
                error!(error = error.to_string(), "Failed to watch the workspace");

                return;
            }

            debug!(path = ?self.workspace_root, "File watcher started");
        } else {
            let root = self.workspace_root.clone();
            let dirs = self.watch_tree(&root, false);

            debug!(path = ?root, dirs, "File watcher started");
        }

        while let Ok(WatcherMessage::Events(result)) = rx.recv() {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            match result {
                Ok(events) => {
                    for event in events {
                        self.handle_event(event);
                    }
                }
                Err(errors) => {
                    for error in errors {
                        warn!(error = error.to_string(), "File watcher error");
                    }
                }
            }
        }

        debug!("File watcher shutting down");
    }

    fn handle_event(&mut self, event: DebouncedEvent) {
        for path in &event.paths {
            self.broadcast(path, event.kind);
        }

        if self.watches_recursively {
            return;
        }

        // Events were dropped (the kernel queue overflowed), possibly
        // for created directories, so ensure everything is watched
        if event.need_rescan() {
            let root = self.workspace_root.clone();

            self.watch_tree(&root, false);

            return;
        }

        // Watch directories that were created or moved into the workspace
        let maybe_dir = match event.kind {
            EventKind::Create(kind) => kind != CreateKind::File,
            EventKind::Modify(ModifyKind::Name(mode)) => mode != RenameMode::From,
            _ => false,
        };

        if maybe_dir
            && let Some(path) = event.paths.last()
            && fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
            && !self.is_ignored_dir(path)
        {
            self.watch_tree(path, true);
        }
    }

    /// Watch the `root` directory, and every directory below it that isn't
    /// ignored, non-recursively, and return how many were watched.
    ///
    /// The recursive mode of per-directory backends walks and watches everything,
    /// including `node_modules` and build outputs, and follows symlinks, which
    /// revisits the same directories through every link to them. In a large
    /// pnpm workspace that is effectively unbounded, so we walk the tree
    /// ourselves, and prune ignored directories and symlinks.
    ///
    /// When `created` is true, the entries found are broadcast as created,
    /// as they may have been created before their parent was watched.
    fn watch_tree(&mut self, root: &Path, created: bool) -> usize {
        let mut dirs = vec![root.to_path_buf()];
        let mut count = 0;

        while let Some(dir) = dirs.pop() {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            if let Err(error) = self.debouncer.watch(&dir, RecursiveMode::NonRecursive) {
                if matches!(error.kind, ErrorKind::MaxFilesWatch) {
                    warn!(
                        dir = ?dir,
                        "Reached the system limit of file watches, so this directory and others are not being watched; ignore large directories with .gitignore, or increase the limit (fs.inotify.max_user_watches on Linux)",
                    );

                    break;
                }

                if dir == self.workspace_root {
                    error!(error = error.to_string(), "Failed to watch the workspace");

                    break;
                }

                // Likely removed while walking
                trace!(dir = ?dir, error = error.to_string(), "Failed to watch directory");

                continue;
            }

            count += 1;

            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();

                // Doesn't follow symlinks
                let is_dir = entry.file_type().is_ok_and(|file_type| file_type.is_dir());

                if is_dir {
                    if self.is_ignored_dir(&path) {
                        continue;
                    }

                    dirs.push(path.clone());
                }

                if created {
                    self.broadcast(
                        &path,
                        EventKind::Create(if is_dir {
                            CreateKind::Folder
                        } else {
                            CreateKind::File
                        }),
                    );
                }
            }
        }

        count
    }

    /// Returns `true` if the directory should not be watched, because it's
    /// always ignored (`node_modules`, etc), or ignored by the VCS.
    fn is_ignored_dir(&self, dir: &Path) -> bool {
        dir.strip_prefix(&self.workspace_root).is_ok_and(is_ignored) || self.vcs.is_ignored(dir)
    }

    fn broadcast(&self, path: &Path, kind: EventKind) {
        if let Some(file_event) = create_file_event(&self.workspace_root, path, kind)
            // We only care about mutations, not access, etc
            && file_event.is_mutated()
        {
            trace!(
                path = ?file_event.path,
                kind = ?file_event.kind,
                "File change event",
            );

            // Ignore send failures
            let _ = self.event_tx.send(file_event);
        }
    }
}

/// Start watching the workspace root for file changes.
///
/// File events are debounced and broadcast on `event_tx`. The OS watcher
/// is owned by a dedicated thread, which runs until `shutdown_rx` receives
/// a message, or this future is dropped (aborted).
///
/// Errors from the `notify` backend are logged but do not stop the
/// watcher — only a shutdown signal does.
pub async fn start_file_watcher(
    workspace_root: PathBuf,
    vcs: Arc<BoxedVcs>,
    event_tx: broadcast::Sender<FileEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> miette::Result<()> {
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let events_tx = tx.clone();

    let debouncer = new_debouncer_opt::<_, RecommendedWatcher, NoCache>(
        DEBOUNCE_TIMEOUT,
        None,
        // This closure runs on notify's internal thread
        move |result| {
            // Receiver dropped — watcher is shutting down
            let _ = events_tx.send(WatcherMessage::Events(result));
        },
        // The default cache (macOS and Windows) records the ID of every file in
        // the workspace, following symlinks, which is unbounded in a pnpm
        // workspace. It's only used to pair the paths of rename events.
        NoCache,
        Config::default(),
    )
    .map_err(map_notify_error)?;

    let watcher = WatcherThread {
        debouncer,
        event_tx,
        stop: Arc::clone(&stop),
        vcs,
        workspace_root,
        watches_recursively: matches!(
            Debouncer::<RecommendedWatcher, NoCache>::kind(),
            WatcherKind::Fsevent | WatcherKind::ReadDirectoryChangesWatcher
        ),
    };

    // Registering watches blocks until complete, which takes a while in a
    // large workspace, so run it on a dedicated thread instead of a runtime
    // worker, as the runtime can't shut down while a worker is blocked.
    // The thread isn't joined, and exits after the stop signal, or with
    // the process.
    thread::Builder::new()
        .name("moon-file-watcher".into())
        .spawn(move || watcher.run(rx))
        .map_err(|error| map_notify_error(notify::Error::io(error)))?;

    let _stop = StopWatcherOnDrop { stop, tx };

    let _ = shutdown_rx.recv().await;

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
            error!(error = format_error_chain(&error), "System watcher error");
        }
    }

    loop {
        tokio::select! {
            result = event_rx.recv() => {
                match result {
                    Ok(event) => {
                        for watcher in watchers.iter_mut() {
                            if let Err(error) = watcher.on_file_event(state.clone(), &event).await {
                                error!(error = format_error_chain(&error), "System watcher error");
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
