use crate::process_error::ProcessError;

pub use std::process::Output;

#[inline]
pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}

#[inline]
pub fn output_to_trimmed_string(data: &[u8]) -> String {
    output_to_string(data).trim().to_owned()
}

pub fn output_to_error(bin: String, output: &Output, with_message: bool) -> ProcessError {
    let code = output.status.code().unwrap_or(-1);

    if !with_message {
        return ProcessError::ExitNonZero { bin, code };
    }

    let mut message = output_to_trimmed_string(&output.stderr);

    if message.is_empty() {
        message = output_to_trimmed_string(&output.stdout);
    }

    ProcessError::ExitNonZeroWithOutput {
        bin,
        code,
        output: message,
    }
}
