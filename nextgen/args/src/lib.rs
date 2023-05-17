use thiserror::Error;

#[derive(Error, Debug)]
#[error("Failed to split arguments `{args}`: {error}")]
pub struct ArgsSplitError {
    args: String,
    error: String,
}

// When parsing a command line with multiple commands separated by a semicolon,
// like "mkdir foo; cd foo", the semicolon is considered part of the leading argument
// if there's no space between them. This attempts to pad the space.
fn pad_semicolon(line: &str) -> String {
    line.replace("; ", " ; ")
}

#[cfg(not(windows))]
pub fn split_args<T: AsRef<str>>(line: T) -> Result<Vec<String>, ArgsSplitError> {
    let line = pad_semicolon(line.as_ref());

    shell_words::split(&line).map_err(|error| ArgsSplitError {
        args: line.to_owned(),
        error: error.to_string(),
    })
}

#[cfg(windows)]
pub fn split_args<T: AsRef<str>>(line: T) -> Result<Vec<String>, ArgsSplitError> {
    let line = pad_semicolon(line.as_ref());

    Ok(winsplit::split(&line))
}

pub fn join_args<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut line = shell_words::join(args);

    // Using `join_args` here incorrectly quotes ";" and other
    // characters, breaking multi-commands.
    if line.contains(" ';' ") {
        line = line.replace("';'", ";");
    }

    if line.contains(" '&&' ") {
        line = line.replace("'&&'", "&&");
    }

    line
}
