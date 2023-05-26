use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
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

pub fn split_args<T: AsRef<str>>(line: T) -> Result<Vec<String>, ArgsSplitError> {
    let line = pad_semicolon(line.as_ref());

    shell_words::split(&line).map_err(|error| ArgsSplitError {
        args: line.to_owned(),
        error: error.to_string(),
    })
}

// #[cfg(windows)]
// pub fn split_args<T: AsRef<str>>(line: T) -> Result<Vec<String>, ArgsSplitError> {
//     let line = pad_semicolon(line.as_ref());

//     Ok(winsplit::split(&line))
// }

// Using `shell_words::join` here incorrectly quotes ";" and other
// characters, breaking multi-commands.
pub fn join_args<I, S>(args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut line = args.into_iter().fold(String::new(), |mut line, arg| {
        let arg = arg.as_ref();

        match arg {
            "&" | "&&" | "|" | "||" | ";" | "!" | ">" | ">>" | "<" => {
                line.push_str(arg);
                line.push(' ');
            }
            _ => {
                let quoted = shell_words::quote(arg);
                line.push_str(quoted.as_ref());
                line.push(' ');
            }
        };

        line
    });

    line.pop();
    line
}
