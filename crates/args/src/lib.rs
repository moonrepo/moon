use miette::Diagnostic;
use std::ffi::{OsStr, OsString};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[diagnostic(code(args::split))]
#[error("Failed to split arguments \"{args}\".")]
pub struct ArgsSplitError {
    args: String,
    #[source]
    error: Box<shell_words::ParseError>,
}

// When parsing a command line with multiple commands separated by a semicolon,
// like "mkdir foo; cd foo", the semicolon is considered part of the leading argument
// if there's no space between them. This attempts to pad the space.
fn pad_semicolon(line: &str) -> String {
    line.replace("; ", " ; ")
}

pub fn split_args<T: AsRef<str>>(line: T) -> miette::Result<Vec<String>> {
    let line = pad_semicolon(line.as_ref());

    Ok(shell_words::split(&line).map_err(|error| ArgsSplitError {
        args: line.to_owned(),
        error: Box::new(error),
    })?)
}

// #[cfg(windows)]
// pub fn split_args<T: AsRef<str>>(line: T) -> miette::Result<Vec<String>> {
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
            "&" | "&&" | "|&" | "|" | "||" | ";" | "!" | ">" | ">>" | "<" | "<<" | "-" | "--" => {
                line.push_str(arg);
                line.push(' ');
            }
            _ => {
                if
                // env var
                arg.starts_with('$') ||
                // already quoted
                arg.starts_with('\'') || arg.starts_with('"') ||
                // glob expansion
                arg.contains('*') || arg.contains('[') || arg.contains('{') || arg.contains('?')
                {
                    line.push_str(arg);
                } else {
                    let quoted = shell_words::quote(arg);
                    line.push_str(quoted.as_ref());
                }

                line.push(' ');
            }
        };

        line
    });

    line.pop(); // Trailing space
    line
}

pub fn join_args_os<I, S>(args: I) -> OsString
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let single_chars = [b"&", b"|", b";", b"!", b">", b"<", b"-"];
    let multi_chars = [b"&&", b"|&", b"||", b">>", b"<<", b"--"];

    let args = args.into_iter().collect::<Vec<_>>();
    let last_index = args.len() - 1;
    let mut index = 0;

    args.into_iter().fold(OsString::new(), |mut line, arg| {
        let arg = arg.as_ref();
        let bytes = arg.as_encoded_bytes();
        let bytes_len = bytes.len();

        // Better way to do this?
        let has_special_chars =
            // Multi chars
            bytes_len > 0 && bytes
                .windows(bytes_len)
                .any(|window| multi_chars.iter().any(|c| *c == window))
            ||
            // Single chars
            single_chars.iter().any(|c| bytes.contains(&c[0]));

        if has_special_chars
            // env var
            || bytes.starts_with(b"$")
            // already quoted
            || bytes.starts_with(b"'")
            || bytes.starts_with(b"\"")
            // glob expansion
            || bytes.contains(&b'*')
            || bytes.contains(&b'[')
            || bytes.contains(&b'{')
            || bytes.contains(&b'?')
        {
            line.push(arg);
        } else {
            let quoted = shell_words::quote(arg.to_str().unwrap()); // Handle conversion?
            line.push(OsStr::new(quoted.as_ref()));
        }

        if index != last_index {
            line.push(OsStr::new(" "));
            index += 1;
        }

        line
    })
}
