pub enum TaskResultStatus {
    Cancelled,
    Failed,
    Passed,
    Pending,
    Running,
}

pub struct TaskResult {
    start_time: i64,

    status: TaskResultStatus,

    end_time: i64,

    exit_code: u8,

    stderr: String,

    stdout: String,
}
