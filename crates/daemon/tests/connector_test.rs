use moon_daemon::{DaemonConnector, write_pid};
use starbase_sandbox::{Sandbox, create_empty_sandbox};
use starbase_utils::fs;

mod connector {
    use super::*;

    fn make_connector(sandbox: &Sandbox) -> DaemonConnector {
        DaemonConnector {
            daemon_dir: sandbox.path().join("daemon"),
            workspace_root: sandbox.path().to_path_buf(),
        }
    }

    #[test]
    fn test_get_log_file() {
        let sandbox = create_empty_sandbox();
        let connector = make_connector(&sandbox);

        assert_eq!(
            connector.get_log_file(),
            sandbox.path().join("daemon/server.log")
        );
    }

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
