// Standard I/O hardening for moon's CLI.
//
// Some environments (notably the GitHub Actions log forwarder) hand
// child processes stdio file descriptors with `O_NONBLOCK` set on the
// underlying open file description. Rust's standard library — and any
// crate that calls `write_all` on `io::stdout()` / `io::stderr()` —
// assumes blocking semantics. Under sustained output, a non-blocking
// stdout pipe returns `EAGAIN` (`io::ErrorKind::WouldBlock`) which
// propagates as a fatal error, causing moon to exit 1 mid-task with
// `Error: console::write_failed`.
//
// See: moonrepo/moon#2465 and the EAGAIN follow-up. starbase's console
// writer was patched separately to retry on `WouldBlock`, but moon
// prints from many writers (`println!`, panic hook, `tracing`,
// `miette`, third-party crates) that we can't audit individually.
// Clearing `O_NONBLOCK` once at startup immunizes them all.

#[cfg(unix)]
pub fn normalize_stdio_blocking() {
    use std::os::raw::c_int;

    for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
        clear_nonblock_on_fd(fd as c_int);
    }
}

#[cfg(not(unix))]
pub fn normalize_stdio_blocking() {}

#[cfg(unix)]
fn clear_nonblock_on_fd(fd: std::os::raw::c_int) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        if flags < 0 {
            return;
        }
        if (flags & libc::O_NONBLOCK) != 0 {
            let _ = libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::raw::c_int;

    fn get_flags(fd: c_int) -> c_int {
        unsafe { libc::fcntl(fd, libc::F_GETFL, 0) }
    }

    fn set_nonblock(fd: c_int) {
        let flags = get_flags(fd);
        assert!(flags >= 0);
        let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        assert_eq!(rc, 0);
    }

    fn make_pipe() -> (c_int, c_int) {
        let mut fds = [0_i32; 2];
        let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
        assert_eq!(rc, 0);
        (fds[0], fds[1])
    }

    fn close(fd: c_int) {
        unsafe { libc::close(fd) };
    }

    // Regression for moon #2465 EAGAIN variant: an fd that arrived with
    // O_NONBLOCK set must come out blocking after normalization.
    #[test]
    fn clears_o_nonblock_when_set() {
        let (r, w) = make_pipe();
        set_nonblock(w);
        assert_ne!(get_flags(w) & libc::O_NONBLOCK, 0, "precondition");

        clear_nonblock_on_fd(w);

        assert_eq!(
            get_flags(w) & libc::O_NONBLOCK,
            0,
            "O_NONBLOCK should be cleared"
        );

        close(r);
        close(w);
    }

    // Idempotent on already-blocking fds: must not flip them or error.
    #[test]
    fn leaves_blocking_fd_alone() {
        let (r, w) = make_pipe();
        let before = get_flags(w);
        assert_eq!(before & libc::O_NONBLOCK, 0, "precondition");

        clear_nonblock_on_fd(w);

        let after = get_flags(w);
        assert_eq!(before, after, "flags should be unchanged");

        close(r);
        close(w);
    }

    // Bad fd must not crash — fcntl returns -1, we ignore.
    #[test]
    fn bad_fd_is_silent() {
        clear_nonblock_on_fd(-1);
        clear_nonblock_on_fd(99999);
    }

    // Smoke test of the public entry point: must not panic regardless
    // of inherited stdio state, and must leave fds 0/1/2 blocking.
    #[test]
    fn normalize_stdio_blocking_clears_inherited_nonblock() {
        // We can't safely flip the real stdio fds in the test process
        // (it would affect cargo's own output). Instead, validate the
        // observable post-condition: after the call, fds 0/1/2 are
        // either blocking or were not modifiable (e.g. not yet open).
        normalize_stdio_blocking();

        for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            let flags = get_flags(fd);
            if flags >= 0 {
                assert_eq!(
                    flags & libc::O_NONBLOCK,
                    0,
                    "fd {fd} should be blocking after normalize_stdio_blocking"
                );
            }
        }
    }
}
