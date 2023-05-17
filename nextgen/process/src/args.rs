use crate::process_error::ProcessError;

pub use shell_words::join;

#[cfg(not(windows))]
pub fn split<T: AsRef<str>>(line: T) -> Result<Vec<String>, ProcessError> {
    let line = line.as_ref();

    shell_words::split(line).map_err(|error| ProcessError::ArgsSplit {
        args: line.to_owned(),
        error: error.to_string(),
    })
}

#[cfg(windows)]
pub fn split<T: AsRef<str>>(line: T) -> Result<Vec<String>, ProcessError> {
    Ok(winsplit::split(line.as_ref()))
}
