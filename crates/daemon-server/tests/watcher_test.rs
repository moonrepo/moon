// Tests for the file watcher that broadcasts file system events, and the
// file listener that dispatches events to registered watchers.

use async_trait::async_trait;
use moon_daemon_server::{start_file_listener, start_file_watcher};
use moon_file_watcher::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast;

// -- Test helpers -----------------------------------------------------------

/// Minimal state that watchers mutate to prove they were called.
#[derive(Clone, Default)]
struct TestState {
    events: Arc<Mutex<Vec<String>>>,
}

/// A watcher that records every event path it receives.
struct RecordingWatcher;

#[async_trait]
impl FileWatcher<TestState> for RecordingWatcher {
    async fn on_init(&mut self, _state: TestState) -> miette::Result<()> {
        Ok(())
    }

    async fn on_file_event(&mut self, state: TestState, event: &FileEvent) -> miette::Result<()> {
        state
            .events
            .lock()
            .unwrap()
            .push(event.path.as_str().to_owned());
        Ok(())
    }
}

/// A watcher that always returns an error.
struct FailingWatcher;

#[async_trait]
impl FileWatcher<TestState> for FailingWatcher {
    async fn on_init(&mut self, _state: TestState) -> miette::Result<()> {
        Ok(())
    }

    async fn on_file_event(&mut self, _state: TestState, _event: &FileEvent) -> miette::Result<()> {
        Err(miette::miette!("deliberate failure"))
    }
}

/// A second recording watcher to verify multiple watchers are dispatched.
struct SecondRecordingWatcher {
    tag: &'static str,
}

#[async_trait]
impl FileWatcher<TestState> for SecondRecordingWatcher {
    async fn on_init(&mut self, _state: TestState) -> miette::Result<()> {
        Ok(())
    }

    async fn on_file_event(&mut self, state: TestState, event: &FileEvent) -> miette::Result<()> {
        state
            .events
            .lock()
            .unwrap()
            .push(format!("{}:{}", self.tag, event.path));
        Ok(())
    }
}

fn make_event(path: &str) -> FileEvent {
    FileEvent {
        path_original: PathBuf::from(path),
        path: path.into(),
        kind: EventKind::Any,
    }
}

// -- Tests ------------------------------------------------------------------

mod file_listener {
    use super::*;

    #[tokio::test]
    async fn test_dispatches_events_to_watcher() {
        let state = TestState::default();
        let events = state.events.clone();

        let (event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let watchers: Vec<BoxedFileWatcher<TestState>> = vec![Box::new(RecordingWatcher)];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        // Give the listener a moment to start
        tokio::time::sleep(Duration::from_millis(20)).await;

        event_tx.send(make_event("src/main.rs")).unwrap();
        event_tx.send(make_event("Cargo.toml")).unwrap();

        // Let events propagate
        tokio::time::sleep(Duration::from_millis(50)).await;

        let _ = shutdown_tx.send(());
        handle.await.unwrap();

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], "src/main.rs");
        assert_eq!(recorded[1], "Cargo.toml");
    }

    #[tokio::test]
    async fn test_dispatches_to_multiple_watchers() {
        let state = TestState::default();
        let events = state.events.clone();

        let (event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let watchers: Vec<BoxedFileWatcher<TestState>> = vec![
            Box::new(RecordingWatcher),
            Box::new(SecondRecordingWatcher { tag: "second" }),
        ];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        tokio::time::sleep(Duration::from_millis(20)).await;

        event_tx.send(make_event("lib.rs")).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let _ = shutdown_tx.send(());
        handle.await.unwrap();

        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], "lib.rs");
        assert_eq!(recorded[1], "second:lib.rs");
    }

    #[tokio::test]
    async fn test_watcher_error_does_not_stop_listener() {
        let state = TestState::default();
        let events = state.events.clone();

        let (event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        // Failing watcher first, recording watcher second
        let watchers: Vec<BoxedFileWatcher<TestState>> =
            vec![Box::new(FailingWatcher), Box::new(RecordingWatcher)];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        tokio::time::sleep(Duration::from_millis(20)).await;

        event_tx.send(make_event("a.rs")).unwrap();
        event_tx.send(make_event("b.rs")).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let _ = shutdown_tx.send(());
        handle.await.unwrap();

        // The recording watcher should still have received both events
        let recorded = events.lock().unwrap();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], "a.rs");
        assert_eq!(recorded[1], "b.rs");
    }

    #[tokio::test]
    async fn test_shutdown_signal_stops_listener() {
        let state = TestState::default();

        let (_event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let watchers: Vec<BoxedFileWatcher<TestState>> = vec![Box::new(RecordingWatcher)];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        tokio::time::sleep(Duration::from_millis(20)).await;

        let _ = shutdown_tx.send(());

        // The listener should exit promptly
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Listener did not shut down in time");
    }

    #[tokio::test]
    async fn test_closed_event_channel_stops_listener() {
        let state = TestState::default();

        let (event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (_shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let watchers: Vec<BoxedFileWatcher<TestState>> = vec![Box::new(RecordingWatcher)];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        tokio::time::sleep(Duration::from_millis(20)).await;

        // Drop the sender to close the channel
        drop(event_tx);

        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Listener did not exit after channel closed");
    }

    #[tokio::test]
    async fn test_no_watchers_still_runs() {
        let state = TestState::default();

        let (event_tx, event_rx) = broadcast::channel::<FileEvent>(16);
        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);

        let watchers: Vec<BoxedFileWatcher<TestState>> = vec![];

        let handle = tokio::spawn(start_file_listener(state, watchers, event_rx, shutdown_rx));

        tokio::time::sleep(Duration::from_millis(20)).await;

        // Should not panic with no watchers
        event_tx.send(make_event("foo.rs")).unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let _ = shutdown_tx.send(());

        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok());
    }
}

mod file_watcher {
    use super::*;
    use moon_vcs::{BoxedVcs, git::Git};
    use starbase_sandbox::{Sandbox, create_empty_sandbox};
    use std::fs;
    use tokio::task::JoinHandle;
    use tokio::time::{Instant, timeout};

    const EVENT_TIMEOUT: Duration = Duration::from_secs(10);

    struct TestFileWatcher {
        event_rx: broadcast::Receiver<FileEvent>,
        handle: JoinHandle<miette::Result<()>>,
        root: PathBuf,
        shutdown_tx: broadcast::Sender<()>,
    }

    impl TestFileWatcher {
        async fn start(sandbox: &Sandbox) -> Self {
            // FSEvents reports real paths, and the temp dir is behind a symlink on macOS
            let root = if cfg!(windows) {
                sandbox.path().to_path_buf()
            } else {
                sandbox.path().canonicalize().unwrap()
            };

            let vcs: BoxedVcs = Box::new(Git::load(&root, "master", &[]).unwrap());
            let (event_tx, event_rx) = broadcast::channel(1024);
            let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

            let handle = tokio::spawn(start_file_watcher(
                root.clone(),
                Arc::new(vcs),
                event_tx,
                shutdown_rx,
            ));

            let mut watcher = Self {
                event_rx,
                handle,
                root,
                shutdown_tx,
            };

            watcher.wait_until_ready().await;
            watcher
        }

        /// Watches are registered in the background, without a signal for
        /// when they're ready, so write a file until its event is received.
        async fn wait_until_ready(&mut self) {
            let deadline = Instant::now() + Duration::from_secs(30);

            for attempt in 0.. {
                assert!(Instant::now() < deadline, "File watcher never became ready");

                fs::write(self.root.join("ready.txt"), format!("{attempt}")).unwrap();

                if self
                    .try_receive("ready.txt", Duration::from_secs(1))
                    .await
                    .is_some()
                {
                    break;
                }
            }
        }

        /// Receive events until one for `path` arrives, and return the paths
        /// of all events received, or `None` if it doesn't arrive in time.
        async fn try_receive(&mut self, path: &str, duration: Duration) -> Option<Vec<String>> {
            let mut paths = vec![];

            let found = timeout(duration, async {
                loop {
                    match self.event_rx.recv().await {
                        Ok(event) => {
                            paths.push(event.path.to_string());

                            if event.path.as_str() == path {
                                return true;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => return false,
                    }
                }
            })
            .await;

            if found == Ok(true) { Some(paths) } else { None }
        }

        async fn receive(&mut self, path: &str) -> Vec<String> {
            self.try_receive(path, EVENT_TIMEOUT)
                .await
                .unwrap_or_else(|| panic!("No event received for {path}"))
        }

        async fn wait_until_closed(&mut self) {
            let closed = timeout(EVENT_TIMEOUT, async {
                while !matches!(
                    self.event_rx.recv().await,
                    Err(broadcast::error::RecvError::Closed)
                ) {}
            })
            .await;

            assert!(closed.is_ok(), "File watcher thread did not stop");
        }

        fn write(&self, path: &str) {
            let path = self.root.join(path);

            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "changed").unwrap();
        }
    }

    #[tokio::test]
    async fn reports_changes_to_files() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file("src/index.ts", "");

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        watcher.write("src/index.ts");
        watcher.receive("src/index.ts").await;
    }

    #[tokio::test]
    async fn reports_changes_in_directories_created_after_starting() {
        let sandbox = create_empty_sandbox();

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        // Created before its parent directories can be watched
        watcher.write("packages/foo/src/moon.yml");
        watcher.receive("packages/foo/src/moon.yml").await;

        // Created after they're watched
        watcher.write("packages/foo/src/index.ts");
        watcher.receive("packages/foo/src/index.ts").await;
    }

    #[tokio::test]
    async fn doesnt_report_changes_in_ignored_directories() {
        let sandbox = create_empty_sandbox();
        sandbox.create_file(".gitignore", "ignored/\n");
        sandbox.create_file(".moon/cache/states/file.json", "");
        sandbox.create_file("ignored/worktree/moon.yml", "");
        sandbox.create_file("node_modules/pkg/index.js", "");

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        watcher.write(".moon/cache/states/file.json");
        watcher.write("ignored/worktree/moon.yml");
        watcher.write("ignored/new/moon.yml");
        watcher.write("node_modules/pkg/index.js");
        watcher.write("node_modules/new/index.js");
        watcher.write("src/index.ts");

        let paths = watcher.receive("src/index.ts").await;

        assert!(
            !paths
                .iter()
                .any(|path| path.starts_with(".moon/cache") || path.starts_with("node_modules")),
            "{paths:?}"
        );

        // Backends with a watch per directory (inotify) don't watch VCS ignored
        // directories, while recursive backends (macOS, Windows) only filter
        // the always ignored directories above
        #[cfg(target_os = "linux")]
        assert!(
            !paths.iter().any(|path| path.starts_with("ignored")),
            "{paths:?}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn doesnt_follow_symlinks() {
        // Like a pnpm workspace, where every package links to the packages it
        // depends on. Following the links visits a package through every path
        // in the dependency graph, which is 2^22 paths for the first package.
        let sandbox = create_empty_sandbox();

        for i in 0..24 {
            sandbox.create_file(format!("packages/p{i}/src/index.ts"), "");

            let deps_dir = sandbox.path().join(format!("packages/p{i}/deps"));

            fs::create_dir_all(&deps_dir).unwrap();

            for j in 0..i {
                std::os::unix::fs::symlink(format!("../../p{j}"), deps_dir.join(format!("p{j}")))
                    .unwrap();
            }
        }

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        watcher.write("packages/p0/src/index.ts");

        let paths = watcher.receive("packages/p0/src/index.ts").await;

        assert!(
            !paths.iter().any(|path| path.contains("/deps/")),
            "{paths:?}"
        );
    }

    #[tokio::test]
    async fn stops_on_shutdown() {
        let sandbox = create_empty_sandbox();

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        watcher.shutdown_tx.send(()).unwrap();

        assert!(
            timeout(Duration::from_secs(2), &mut watcher.handle)
                .await
                .is_ok(),
            "File watcher did not shut down in time"
        );

        watcher.wait_until_closed().await;
    }

    #[tokio::test]
    async fn stops_when_aborted() {
        let sandbox = create_empty_sandbox();

        let mut watcher = TestFileWatcher::start(&sandbox).await;

        watcher.handle.abort();
        watcher.wait_until_closed().await;
    }
}
