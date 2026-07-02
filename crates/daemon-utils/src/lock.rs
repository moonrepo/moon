use std::fs::{File, OpenOptions, TryLockError};
use std::io;
use std::path::Path;

/// An exclusive advisory lock on a file, held for as long as the returned
/// guard is alive and released automatically when it is dropped — or when the
/// process exits, including on a crash or `SIGKILL`, since the kernel drops
/// the lock when the file descriptor closes. This is what lets the daemon
/// own its endpoint files without leaving stale state behind: liveness is the
/// lock, not a PID that may have been reused or a process that may be a zombie.
///
/// Backed by `flock` on Unix and `LockFileEx` on Windows via the standard
/// library. The lock lives on the open file description, so the process
/// holding this guard owns it while other processes (and other handles, even
/// in the same process) observe it as contended.
#[derive(Debug)]
pub struct DaemonLock {
    // Held to keep the lock alive; the lock releases when this file closes.
    file: File,
}

impl DaemonLock {
    /// Try to acquire the lock without blocking.
    ///
    /// - `Ok(Some(lock))` — acquired; hold the guard to keep ownership.
    /// - `Ok(None)` — currently held by another handle or process.
    /// - `Err(_)` — the lock file could not be opened or locked.
    pub fn try_acquire(path: &Path) -> io::Result<Option<Self>> {
        // Windows requires read or write access (not append-only) to lock.
        // The lock file's contents are irrelevant — never truncate it.
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        match file.try_lock() {
            Ok(()) => Ok(Some(Self { file })),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        // Best-effort; closing the file releases the lock regardless.
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starbase_sandbox::create_empty_sandbox;

    #[test]
    fn acquires_when_free() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("daemon.lock");

        let lock = DaemonLock::try_acquire(&path).unwrap();

        assert!(lock.is_some());
    }

    #[test]
    fn contends_while_held_then_frees_on_drop() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("daemon.lock");

        let first = DaemonLock::try_acquire(&path).unwrap();
        assert!(first.is_some());

        // A second acquisition (a distinct open file description) sees the
        // lock as held, even within the same process.
        let second = DaemonLock::try_acquire(&path).unwrap();
        assert!(second.is_none());

        // Dropping the holder releases the lock, so the next attempt succeeds.
        drop(first);

        let third = DaemonLock::try_acquire(&path).unwrap();
        assert!(third.is_some());
    }

    #[test]
    fn creates_lock_file_if_missing() {
        let sandbox = create_empty_sandbox();
        let path = sandbox.path().join("nested").join("daemon.lock");

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let _lock = DaemonLock::try_acquire(&path).unwrap().unwrap();

        assert!(path.exists());
    }
}
