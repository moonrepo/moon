// Tests for the file listener that dispatches events to registered watchers.

use async_trait::async_trait;
use moon_daemon_server::start_file_listener;
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
