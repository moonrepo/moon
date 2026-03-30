use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

pub use tokio_stream::wrappers::UnixListenerStream;

pub fn is_process_alive(pid: u32) -> bool {
    // Reject invalid PIDs (0 = all processes in group, negative = process groups)
    let pid = pid as libc::pid_t;

    if pid <= 0 {
        return false;
    }

    // Signal 0 doesn't send a signal but checks process existence
    unsafe { libc::kill(pid, 0) == 0 }
}

pub fn kill_process(pid: u32) -> std::io::Result<()> {
    let ret = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };

    if ret != 0 {
        let error = std::io::Error::last_os_error();

        // ESRCH means the process is already gone — not an error.
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error);
        }
    }

    Ok(())
}

pub fn create_detached_command(exe: &Path) -> Command {
    let mut command = Command::new(exe);

    unsafe {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Create a new session so the daemon survives the parent exiting.
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            });
    };

    command
}
