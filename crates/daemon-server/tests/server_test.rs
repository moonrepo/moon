// Integration tests: spin up a real daemon server over a platform-specific
// transport, connect a client, and exercise every RPC method.

use moon_daemon_client::DaemonClient;
use moon_daemon_server::*;
use moon_daemon_utils::endpoint::*;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::broadcast;

pub fn build_daemon_service(
    workspace_root: PathBuf,
    moon_version: String,
    endpoint: String,
    pid: u32,
    shutdown_tx: broadcast::Sender<()>,
) -> DaemonService {
    DaemonService::new(workspace_root, moon_version, endpoint, pid, shutdown_tx)
}

#[cfg(unix)]
mod unix_rpc {
    use super::*;
    use moon_daemon_server::serve_unix;

    /// Helper: start a gRPC server in the background on a temporary UDS,
    /// returning a shutdown sender so the test can stop it.
    async fn start_test_server(daemon_dir: &Path, workspace_root: &Path) -> broadcast::Sender<()> {
        let endpoint = get_endpoint(daemon_dir);
        let pid = std::process::id();
        let pid_path = get_pid_path(daemon_dir);

        write_pid(&pid_path, pid).unwrap();

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let shutdown_signal = async move {
            let _ = shutdown_rx.recv().await;
        };

        let service = build_daemon_service(
            workspace_root.to_owned(),
            "0.0.0-test".to_owned(),
            endpoint.clone(),
            pid,
            shutdown_tx.clone(),
        );

        tokio::spawn(async move {
            serve_unix(&endpoint, service, shutdown_signal)
                .await
                .unwrap();
        });

        // Give the server a moment to bind.
        tokio::time::sleep(Duration::from_millis(50)).await;

        shutdown_tx
    }

    #[tokio::test]
    async fn test_status_rpc() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let status = client.status().await.unwrap();

        assert!(status.running);
        assert_eq!(status.pid, std::process::id());
        assert_eq!(status.moon_version, "0.0.0-test");
        assert_eq!(status.workspace_root, workspace_root.to_string_lossy());
        assert!(status.uptime_secs < 5); // should be nearly instant

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_start_rpc_returns_already_running() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let response = client
            .start(&workspace_root.to_string_lossy())
            .await
            .unwrap();

        assert!(response.already_running);
        assert_eq!(response.pid, std::process::id());
        assert!(!response.endpoint.is_empty());

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_stop_rpc() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let _shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let response = client.stop().await.unwrap();

        assert!(response.stopped);
    }

    #[tokio::test]
    async fn test_status_after_multiple_calls() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        // Multiple status calls should all succeed.
        for _ in 0..3 {
            let status = client.status().await.unwrap();
            assert!(status.running);
        }

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_connect_to_nonexistent_socket_fails() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();

        let result = DaemonClient::connect(&daemon_dir).await;

        assert!(result.is_err());
    }
}

#[cfg(windows)]
mod windows_rpc {
    use super::*;
    use moon_daemon_server::serve_windows;

    /// Helper: start a gRPC server in the background on a temporary named pipe,
    /// returning a shutdown sender so the test can stop it.
    async fn start_test_server(daemon_dir: &Path, workspace_root: &Path) -> broadcast::Sender<()> {
        let endpoint = get_endpoint(daemon_dir);
        let pid = std::process::id();
        let pid_path = get_pid_path(daemon_dir);

        write_pid(&pid_path, pid).unwrap();

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        let shutdown_signal = async move {
            let _ = shutdown_rx.recv().await;
        };

        let service = build_daemon_service(
            workspace_root.to_owned(),
            "0.0.0-test".to_owned(),
            endpoint.clone(),
            pid,
            shutdown_tx.clone(),
        );

        tokio::spawn(async move {
            serve_windows(&endpoint, service, shutdown_signal)
                .await
                .unwrap();
        });

        // Give the server a moment to bind.
        tokio::time::sleep(Duration::from_millis(100)).await;

        shutdown_tx
    }

    #[tokio::test]
    async fn test_status_rpc() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let status = client.status().await.unwrap();

        assert!(status.running);
        assert_eq!(status.pid, std::process::id());
        assert_eq!(status.moon_version, "0.0.0-test");
        assert_eq!(status.workspace_root, workspace_root.to_string_lossy());
        assert!(status.uptime_secs < 5);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_start_rpc_returns_already_running() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let response = client
            .start(&workspace_root.to_string_lossy())
            .await
            .unwrap();

        assert!(response.already_running);
        assert_eq!(response.pid, std::process::id());
        assert!(!response.endpoint.is_empty());

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_stop_rpc() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let _shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        let response = client.stop().await.unwrap();

        assert!(response.stopped);
    }

    #[tokio::test]
    async fn test_status_after_multiple_calls() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");
        let workspace_root = sandbox.path().to_path_buf();

        fs::create_dir_all(&daemon_dir).unwrap();

        let shutdown_tx = start_test_server(&daemon_dir, &workspace_root).await;
        let mut client = DaemonClient::connect(&daemon_dir).await.unwrap();

        // Multiple status calls should all succeed.
        for _ in 0..3 {
            let status = client.status().await.unwrap();
            assert!(status.running);
        }

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_connect_to_nonexistent_pipe_fails() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();

        let result = DaemonClient::connect(&daemon_dir).await;

        assert!(result.is_err());
    }
}
