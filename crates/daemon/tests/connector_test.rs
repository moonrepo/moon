use moon_daemon::{DaemonClient, DaemonConnector};
use moon_daemon_utils::endpoint::{DaemonInfo, write_state};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::fs;

fn make_connector(sandbox: &Sandbox) -> DaemonConnector {
    // "0.0.1" matches the version the mocked daemon reports, so the handshake
    // in `acquire` treats an in-test server as a compatible daemon.
    DaemonConnector::new(
        sandbox.path().join("daemon"),
        sandbox.path().to_path_buf(),
        "0.0.1".into(),
    )
}

mod connector {
    use super::*;

    #[test]
    fn test_get_state_file() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        assert_eq!(
            connector.get_state_file(),
            sandbox.path().join("daemon/daemon.json")
        );
    }

    #[test]
    fn test_read_state_none_when_missing() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        assert!(connector.read_state().is_none());
    }

    #[test]
    fn test_read_state_returns_written_info() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();
        write_state(
            &connector.daemon_dir,
            DaemonInfo::new(4242, "1.2.3".into(), "sock".into()),
        )
        .unwrap();

        let state = connector.read_state().unwrap();
        assert_eq!(state.pid, 4242);
        assert_eq!(state.version, "1.2.3");
    }

    #[tokio::test]
    async fn test_is_running_false_when_no_daemon() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Liveness is a connection, not the state file: a stale record must
        // not read as running when nothing is listening.
        write_state(
            &connector.daemon_dir,
            DaemonInfo::new(4_000_000, "1.0.0".into(), "sock".into()),
        )
        .unwrap();

        assert!(!connector.is_running().await);
    }

    #[tokio::test]
    async fn test_stop_daemon_not_running() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Nothing listening and no state file: nothing to stop.
        let result = connector.stop_daemon().await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_stop_daemon_cleans_stale_state_when_lock_free() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // A crashed daemon left a state file but released its ownership lock
        // (nothing is listening, the lock is free). Stop confirms death by
        // acquiring the lock, then removes the stale files.
        write_state(
            &connector.daemon_dir,
            DaemonInfo::new(4_000_000, "1.0.0".into(), "sock".into()),
        )
        .unwrap();

        let result = connector.stop_daemon().await.unwrap();

        assert!(result);
        assert!(!connector.get_state_file().exists());
    }
}

mod connect {
    use super::*;

    #[tokio::test]
    async fn test_try_connect_classifies_missing_endpoint() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // No daemon has ever run here, so the endpoint doesn't exist.
        let error = DaemonClient::try_connect(&connector.daemon_dir)
            .await
            .unwrap_err();

        assert!(error.is_endpoint_unavailable());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_try_connect_classifies_stale_socket() {
        use moon_daemon_utils::endpoint::get_sock_path;

        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Bind then drop the listener, leaving a socket file with nothing
        // accepting on it — the connect is refused, as after a daemon crash.
        drop(tokio::net::UnixListener::bind(get_sock_path(&connector.daemon_dir)).unwrap());

        let error = DaemonClient::try_connect(&connector.daemon_dir)
            .await
            .unwrap_err();

        assert!(error.is_endpoint_unavailable());
    }

    #[tokio::test]
    async fn test_connect_once_fails_fast_when_not_running() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        assert!(connector.connect_once().await.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_connect_retries_until_endpoint_appears() {
        use moon_daemon::{DaemonService, DaemonState, serve_unix};
        use moon_daemon_utils::endpoint::get_endpoint;
        use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::{RwLock, broadcast};

        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        let endpoint = get_endpoint(&connector.daemon_dir);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let mocker = WorkspaceMocker::new(sandbox.path());
        let service = DaemonService::new(
            Arc::new(RwLock::new(DaemonState {
                app_context: Arc::new(mocker.mock_app_context()),
                workspace_graph: Arc::new(WorkspaceGraph::default()),
            })),
            endpoint.clone(),
            std::process::id(),
            shutdown_tx.clone(),
        );

        // Bind the endpoint only after a delay, so the first connect
        // attempts fail with "not found" and must be retried.
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;

            serve_unix(&endpoint, service, async move {
                let _ = shutdown_rx.recv().await;
            })
            .await
            .unwrap();
        });

        let client = connector.connect().await.unwrap();

        assert!(client.is_some());

        let _ = shutdown_tx.send(());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_acquire_connects_to_running_daemon_without_spawning() {
        use moon_daemon::{DaemonService, DaemonState, serve_unix};
        use moon_daemon_utils::endpoint::get_endpoint;
        use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::{RwLock, broadcast};

        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        let endpoint = get_endpoint(&connector.daemon_dir);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let mocker = WorkspaceMocker::new(sandbox.path());
        let service = DaemonService::new(
            Arc::new(RwLock::new(DaemonState {
                app_context: Arc::new(mocker.mock_app_context()),
                workspace_graph: Arc::new(WorkspaceGraph::default()),
            })),
            endpoint.clone(),
            std::process::id(),
            shutdown_tx.clone(),
        );

        tokio::spawn(async move {
            serve_unix(&endpoint, service, async move {
                let _ = shutdown_rx.recv().await;
            })
            .await
            .unwrap();
        });

        // Give the server a moment to bind.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // A daemon is already listening, so acquire connects on the fast path.
        // (If it fell through to spawning, it would try to launch the test
        // binary as `daemon server` and fail.)
        let client = connector.acquire().await.unwrap();

        assert!(client.is_some());

        let _ = shutdown_tx.send(());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_acquire_restarts_version_mismatched_daemon() {
        use moon_daemon::{DaemonConnector, DaemonService, DaemonState, serve_unix};
        use moon_daemon_utils::endpoint::get_endpoint;
        use moon_test_utils::{WorkspaceGraph, WorkspaceMocker};
        use std::sync::Arc;
        use std::time::Duration;
        use tokio::sync::{RwLock, broadcast};

        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();

        let endpoint = get_endpoint(&daemon_dir);
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let mocker = WorkspaceMocker::new(sandbox.path());
        let service = DaemonService::new(
            Arc::new(RwLock::new(DaemonState {
                app_context: Arc::new(mocker.mock_app_context()),
                workspace_graph: Arc::new(WorkspaceGraph::default()),
            })),
            endpoint.clone(),
            std::process::id(),
            shutdown_tx.clone(),
        );

        // The stop RPC drives this shutdown, so the task completing proves the
        // mismatched daemon was told to stop.
        let server = tokio::spawn(async move {
            serve_unix(&endpoint, service, async move {
                let _ = shutdown_rx.recv().await;
            })
            .await
            .unwrap();
        });

        // Give the server a moment to bind.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // The running daemon reports version "0.0.1"; this connector is a
        // different version, so the handshake must reject it.
        let connector =
            DaemonConnector::new(daemon_dir, sandbox.path().to_path_buf(), "999.0.0".into());

        // acquire connects, detects the mismatch, stops the daemon, then tries
        // to respawn — which fails in the test binary — and degrades to None.
        let result = connector.acquire().await.unwrap();

        assert!(result.is_none());

        // The mismatched daemon was stopped (its serve task returned).
        tokio::time::timeout(Duration::from_secs(5), server)
            .await
            .expect("daemon should have been stopped")
            .unwrap();
    }
}
