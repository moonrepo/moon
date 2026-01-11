use crate::command::{Command, CommandExecutable};
use moon_args::join_args_os;
use moon_common::color;
use moon_env_var::GlobalEnvBag;
use std::ffi::OsString;
use std::fmt::{self, Display};
use std::path::Path;

fn should_quote(value: &str) -> bool {
    value.chars().any(|ch| ch.is_ascii_whitespace())
}

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

        // If wrapped in a shell, the shell binary and arguments
        // must be placed at the start of the line
        if let Some(shell) = &command.shell {
            in_shell = true;
            command_line.push(shell.bin.as_os_str().to_owned());
            command_line.extend(shell.command.shell_args.clone());

            // Within a shell, the command is a string. For arguments,
            // use the original quoted value if available!
            let mut shell_command = OsString::new();

            match &command.exe {
                CommandExecutable::Binary(bin) => {
                    let mut args = vec![bin];
                    args.extend(&command.args);

                    for arg in args {
                        if !shell_command.is_empty() {
                            shell_command.push(" ");
                        }

                        if let Some(quoted_value) = &arg.quoted_value {
                            shell_command.push(quoted_value);
                        } else if let Some(value) = arg.value.to_str()
                            && should_quote(value)
                        {
                            shell_command.push(shell.instance.quote(value));
                        } else {
                            shell_command.push(&arg.value);
                        }
                    }
                }
                CommandExecutable::Script(script) => {
                    shell_command.push(script);
                }
            };

            // If the main command should be passed via stdin,
            // then append the input line instead of the command line
            if shell.command.pass_args_stdin {
                input_line.push(shell_command);
            }
            // Otherwise append as a *single* argument. This typically
            // appears after a "-c" argument (should come from shell)
            else {
                command_line.push(shell_command);
            }
        }
        // Otherwise we have a normal command and arguments
        else {
            match &command.exe {
                CommandExecutable::Binary(bin) => {
                    command_line.push(bin.value.clone());

                    for arg in &command.args {
                        command_line.push(arg.value.clone());
                    }
                }
                CommandExecutable::Script(script) => {
                    command_line.push(script.clone());
                }
            };

            // That also may have input
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
        let mut line = OsString::new();

        if !with_shell
            && self.shell
            && let Some(last) = self.command.last()
        {
            line.push(last);
        } else {
            for arg in &self.command {
                if !line.is_empty() {
                    line.push(" ");
                }

                line.push(arg);

                if with_shell && self.shell && (arg == "-c" || arg == "-C") {
                    line.push(" -");
                }
            }
        }

        if with_input && !self.input.is_empty() {
            let debug_input = GlobalEnvBag::instance().should_debug_process_input();
            let input = join_args_os(self.input.iter().flat_map(|i| i.to_str().map(|s| s.trim())));

            if line
                .as_os_str()
                .to_str()
                .is_some_and(|cmd| cmd.ends_with('-'))
            {
                line.push(" ");
            } else {
                line.push(" - ");
            }

            if input.len() > 200 && !debug_input {
                line.push("(truncated input)");
            } else {
                line.push(&input);
            }
        }

        line.to_string_lossy().trim().replace('\n', " ")
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
