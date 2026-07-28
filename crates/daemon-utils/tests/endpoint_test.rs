use moon_daemon_utils::endpoint::*;
use starbase_sandbox::create_empty_sandbox;
use starbase_utils::fs;
use std::path::Path;

mod endpoint {
    use super::*;

    #[test]
    fn test_get_state_path() {
        let daemon_dir = Path::new("/home/user/.moon/daemon");

        assert_eq!(get_state_path(daemon_dir), daemon_dir.join("daemon.json"));
    }

    #[test]
    fn test_get_lock_path() {
        let daemon_dir = Path::new("/home/user/.moon/daemon");

        assert_eq!(get_lock_path(daemon_dir), daemon_dir.join("daemon.lock"));
    }

    #[test]
    fn test_get_spawn_lock_path() {
        let daemon_dir = Path::new("/home/user/.moon/daemon");

        assert_eq!(
            get_spawn_lock_path(daemon_dir),
            daemon_dir.join("spawn.lock")
        );
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
    fn test_read_write_state() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path();

        let info = DaemonInfo::new(12345, "1.2.3".into(), "/tmp/moond.sock".into());
        write_state(daemon_dir, info.clone()).unwrap();

        assert_eq!(read_state(daemon_dir), Some(info));
    }

    #[test]
    fn test_write_state_overwrites_existing() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path();

        write_state(daemon_dir, DaemonInfo::new(111, "1.0.0".into(), "a".into())).unwrap();
        write_state(daemon_dir, DaemonInfo::new(222, "2.0.0".into(), "b".into())).unwrap();

        let state = read_state(daemon_dir).unwrap();
        assert_eq!(state.pid, 222);
        assert_eq!(state.version, "2.0.0");
    }

    #[test]
    fn test_read_state_missing_file() {
        let daemon_dir = Path::new("/nonexistent/path/daemon");

        assert_eq!(read_state(daemon_dir), None);
    }

    #[test]
    fn test_read_state_invalid_content() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path();

        std::fs::write(get_state_path(daemon_dir), "not-json").unwrap();

        assert_eq!(read_state(daemon_dir), None);
    }

    #[test]
    fn test_cleanup_daemon_files() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();
        write_state(
            &daemon_dir,
            DaemonInfo::new(123, "1.0.0".into(), "s".into()),
        )
        .unwrap();
        std::fs::write(get_sock_path(&daemon_dir), "").unwrap();

        cleanup_daemon_files(&daemon_dir).unwrap();

        assert!(!get_state_path(&daemon_dir).exists());
        assert!(!get_sock_path(&daemon_dir).exists());
    }

    #[test]
    fn test_cleanup_daemon_files_leaves_lock_files() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("daemon");

        fs::create_dir_all(&daemon_dir).unwrap();
        std::fs::write(get_lock_path(&daemon_dir), "").unwrap();

        cleanup_daemon_files(&daemon_dir).unwrap();

        // Lock files are reused across runs and must survive cleanup.
        assert!(get_lock_path(&daemon_dir).exists());
    }

    #[test]
    fn test_cleanup_daemon_files_missing_dir() {
        let sandbox = create_empty_sandbox();
        let daemon_dir = sandbox.path().join("nonexistent");

        // Should not panic or error on missing dir.
        assert!(cleanup_daemon_files(&daemon_dir).is_ok());
    }
}
