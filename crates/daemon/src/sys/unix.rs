pub fn is_process_alive(pid: u32) -> bool {
    // Reject invalid PIDs (0 = all processes in group, negative = process groups)
    let pid = pid as libc::pid_t;

    if pid <= 0 {
        return false;
    }

    // Signal 0 doesn't send a signal but checks process existence
    unsafe { libc::kill(pid, 0) == 0 }
}
