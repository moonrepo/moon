use std::ffi::{OsStr, OsString};

use crate::Command;

pub struct CommandInspector<'cmd> {
    /// The entire command line as a list of arguments.
    /// The first argument is always the program/command to run.
    command_line: Vec<&'cmd OsString>,

    /// The entire input line to pass as stdin to a running command.
    input_line: Vec<&'cmd OsString>,

    pub prefix: Option<&'cmd String>,

    /// Whether to error on non-zero status codes.
    pub should_error_nonzero: bool,

    /// Whether to pass the input line as stdin.
    /// Is true when parent command has input, or a shell requires it.
    pub should_pass_stdin: bool,

    // Whether to print the command/input lines to the terminal.
    pub should_print: bool,
}

impl<'cmd> CommandInspector<'cmd> {
    pub fn from(command: &'cmd Command) -> CommandInspector {
        let mut command_line: Vec<&'cmd OsString> = vec![&command.bin];
        let mut input_line: Vec<&'cmd OsString> = vec![];

        if !command.args.is_empty() {
            command_line.extend(&command.args);
        }

        if command.input.is_empty() {
            input_line.push(&command.bin);
            input_line.extend(&command.args);
        } else {
            input_line.extend(&command.input);
        }

        CommandInspector {
            command_line,
            input_line,
            prefix: command.prefix.as_ref(),
            should_error_nonzero: command.error_on_nonzero,
            should_pass_stdin: !command.input.is_empty()
                || command
                    .shell
                    .as_ref()
                    .map(|s| s.pass_args_stdin)
                    .unwrap_or(false),
            should_print: command.print_command,
        }
    }

    pub fn get_input_line(&self) -> Option<String> {
        if !self.should_pass_stdin {
            return None;
        }

        let line = self
            .input_line
            .iter()
            .map(|i| i.as_os_str())
            .collect::<Vec<_>>()
            .join(OsStr::new(" "))
            .to_str()
            .unwrap_or_default()
            .to_string();

        Some(line)
    }
}
