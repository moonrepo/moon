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
        // u32::MAX wraps to -1 as pid_t, which is a special value for kill().
        // Use a very high but valid PID that is extremely unlikely to exist.
        assert!(!is_process_alive(4_000_000));
    }
}
