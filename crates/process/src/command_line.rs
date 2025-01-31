use crate::command::Command;
use moon_args::join_args_os;
use moon_common::color;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display};
use std::path::Path;

#[derive(Debug)]
pub struct CommandLine {
    pub command: Vec<OsString>,
    pub input: Vec<OsString>,
    pub shell: bool,
}

impl CommandLine {
    pub fn new(command: &Command) -> CommandLine {
        let mut command_line: Vec<OsString> = vec![];
        let mut input_line: Vec<OsString> = vec![];
        let mut in_shell = false;

        // Extract the main command, without shell, for other purposes!
        let mut main_line: Vec<OsString> = vec![];
        main_line.push(command.bin.clone());

        for arg in &command.args {
            main_line.push(arg.to_owned());
        }

        // If wrapped in a shell, the shell binary and arguments
        // must be placed at the start of the line.
        if let Some(shell) = &command.shell {
            in_shell = true;
            command_line.push(shell.bin.as_os_str().to_owned());
            command_line.extend(shell.command.shell_args.clone());

            // If the main command should be passed via stdin,
            // then append the input line instead of the command line.
            if shell.command.pass_args_stdin {
                input_line.extend(main_line);
            }
            // Otherwise append as a *single* argument. This typically
            // appears after a "-" argument (should come from shell).
            else {
                command_line.push(if command.escape_args {
                    join_args_os(main_line)
                } else {
                    main_line.join(OsStr::new(" "))
                });
            }

            // Otherwise we have a normal command and arguments.
        } else {
            command_line.extend(main_line);

            // That also may have input.
            if !command.input.is_empty() {
                for input in &command.input {
                    input_line.push(input.to_owned());
                }
            }
        }

        CommandLine {
            command: command_line,
            input: input_line,
            shell: in_shell,
        }
    }

    pub fn get_line(&self, with_shell: bool, with_input: bool) -> String {
        let mut command = if !with_shell && self.shell {
            self.command.last().cloned().unwrap_or_else(OsString::new)
        } else {
            join_args_os(&self.command)
        };

        if with_input && !self.input.is_empty() {
            let debug_input = env::var("MOON_DEBUG_PROCESS_INPUT").is_ok();
            let input = join_args_os(&self.input);

            if command
                .as_os_str()
                .to_str()
                .is_some_and(|cmd| cmd.ends_with('-'))
            {
                command.push(" ");
            } else {
                command.push(" - ");
            }

            if input.len() > 200 && !debug_input {
                command.push("(truncated)");
            } else {
                command.push(&input);
            }
        }

        command.to_string_lossy().trim().replace('\n', " ")
    }

    pub fn format(command: &str, workspace_root: &Path, working_dir: &Path) -> String {
        let dir = if working_dir == workspace_root {
            "workspace".into()
        } else if let Ok(dir) = working_dir.strip_prefix(workspace_root) {
            format!(".{}{}", std::path::MAIN_SEPARATOR, dir.to_string_lossy())
        } else {
            ".".into()
        };

        format!(
            "{} {}",
            color::muted_light(command.trim()),
            color::muted(format!("(in {dir})"))
        )
    }
}

impl Display for CommandLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_line(true, true))
    }
}
