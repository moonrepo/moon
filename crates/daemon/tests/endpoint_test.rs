use moon_daemon::{cleanup_daemon_files, get_endpoint, get_pid_path, read_pid, write_pid};
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::fs;
use std::path::Path;

mod endpoint {
    use super::*;

    #[test]
    fn test_get_pid_path() {
        let daemon_dir = Path::new("/home/user/.moon/daemon");
        let pid_path = get_pid_path(daemon_dir);

        assert_eq!(pid_path, daemon_dir.join("moond.pid"));
    }

    #[cfg(unix)]
    #[test]
    fn test_get_endpoint_unix() {
        let daemon_dir = Path::new("/home/user/.moon/daemon");
        let endpoint = get_endpoint(daemon_dir);

        assert_eq!(endpoint, "/home/user/.moon/daemon/moond.sock");
    }

    #[cfg(windows)]
    #[test]
    fn test_get_endpoint_windows_is_named_pipe() {
        let daemon_dir = Path::new("C:\\Users\\user\\.moon\\daemon");
        let endpoint = get_endpoint(daemon_dir);

        assert!(endpoint.starts_with(r"\\.\pipe\moon-daemon-"));
    }

    #[cfg(windows)]
    #[test]
    fn test_get_endpoint_windows_different_dirs_produce_different_pipes() {
        let ep1 = get_endpoint(Path::new("C:\\a\\.moon\\daemon"));
        let ep2 = get_endpoint(Path::new("C:\\b\\.moon\\daemon"));

        assert_ne!(ep1, ep2);
    }

    #[test]
    fn test_read_write_pid() {
        let sandbox = create_empty_sandbox();
        let pid_path = sandbox.path().join("test.pid");

        write_pid(&pid_path, 12345).unwrap();
        assert_eq!(read_pid(&pid_path), Some(12345));
    }

    #[test]
    fn test_write_pid_overwrites_existing() {
        let sandbox = create_empty_sandbox();
        let pid_path = sandbox.path().join("test.pid");

        write_pid(&pid_path, 111).unwrap();
        write_pid(&pid_path, 222).unwrap();

        assert_eq!(read_pid(&pid_path), Some(222));
    }

    #[test]
    fn test_read_pid_missing_file() {
        let path = Path::new("/nonexistent/path/test.pid");
        assert_eq!(read_pid(path), None);
    }

    #[test]
    fn test_read_pid_invalid_content() {
        let sandbox = create_empty_sandbox();
        let pid_path = sandbox.path().join("bad.pid");

        std::fs::write(&pid_path, "not-a-number").unwrap();

        assert_eq!(read_pid(&pid_path), None);
    }

    #[test]
    fn test_read_pid_empty_file() {
        let sandbox = create_empty_sandbox();
        let pid_path = sandbox.path().join("empty.pid");

        std::fs::write(&pid_path, "").unwrap();

        assert_eq!(read_pid(&pid_path), None);
    }

    #[test]
    fn test_read_pid_with_whitespace() {
        let sandbox = create_empty_sandbox();
        let pid_path = sandbox.path().join("ws.pid");

        std::fs::write(&pid_path, "  42  \n").unwrap();

        assert_eq!(read_pid(&pid_path), Some(42));
    }

    #[test]
    fn test_cleanup_daemon_files() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();
        std::fs::write(daemon_dir.join("moond.pid"), "123").unwrap();
        std::fs::write(daemon_dir.join("moond.sock"), "").unwrap();

        cleanup_daemon_files(&daemon_dir).unwrap();

        assert!(!daemon_dir.exists());
    }

    #[test]
    fn test_cleanup_daemon_files_missing_dir() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("nonexistent");

        // Should not panic or error on missing dir
        let result = cleanup_daemon_files(&daemon_dir);
        assert!(result.is_ok());
    }
}
