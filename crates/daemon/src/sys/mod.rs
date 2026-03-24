#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub use windows::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_process_alive_current() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn test_is_process_alive_dead() {
        // Use a very high PID that is extremely unlikely to exist.
        assert!(!is_process_alive(4_000_000));
    }

    #[test]
    fn test_is_process_alive_zero() {
        // PID 0 is special (kernel/idle) — should return false.
        assert!(!is_process_alive(0));
    }

    #[test]
    fn test_kill_process_nonexistent() {
        // Killing a non-existent process should succeed (ESRCH is ignored).
        let result = kill_process(4_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_detached_command_exists() {
        let exe = std::env::current_exe().unwrap();
        let command = create_detached_command(&exe);

        // Verify the command was created with the right program.
        assert_eq!(command.get_program(), exe.as_os_str());
    }
}
