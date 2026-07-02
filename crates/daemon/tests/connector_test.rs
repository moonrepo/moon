use moon_daemon::{DaemonClient, DaemonConnector};
use moon_daemon_utils::endpoint::write_pid;
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::fs;

fn make_connector(sandbox: &Sandbox) -> DaemonConnector {
    DaemonConnector::new(sandbox.path().join("daemon"), sandbox.path().to_path_buf())
}

mod connector {
    use super::*;

    #[test]
    fn test_get_pid_file() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        assert_eq!(
            connector.get_pid_file(),
            sandbox.path().join("daemon/moond.pid")
        );
    }

    #[test]
    fn test_is_running_no_pid_file() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        assert!(connector.is_running().is_none());
    }

    #[test]
    fn test_is_running_stale_pid() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Write a PID for a process that (almost certainly) doesn't exist.
        let pid_path = connector.get_pid_file();
        write_pid(&pid_path, 4_000_000).unwrap();

        assert!(connector.is_running().is_none());
    }

    #[test]
    fn test_is_running_current_process() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Write our own PID — the process is definitely alive.
        let pid_path = connector.get_pid_file();
        let pid = std::process::id();
        write_pid(&pid_path, pid).unwrap();

        assert_eq!(connector.is_running(), Some(pid));
    }

    #[test]
    fn test_is_running_no_daemon_dir() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        // daemon_dir doesn't exist at all.
        assert!(connector.is_running().is_none());
    }

    #[tokio::test]
    async fn test_stop_daemon_not_running() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        fs::create_dir_all(&connector.daemon_dir).unwrap();

        // Stopping when nothing is running should return Ok(false).
        let result = connector.stop_daemon().await.unwrap();
        assert!(!result);
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
}
