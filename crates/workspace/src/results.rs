pub enum ResultStatus {
    Cancelled,
    Failed,
    Passed,
    Pending,
    Running,
}

pub struct Result {
    start_time: i64,

    status: ResultStatus,

    end_time: i64,

    exit_code: u8,

    stderr: String,

    stdout: String,
}
