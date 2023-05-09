use crate::command::Command;
use moon_common::color;
use rustc_hash::FxHashMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use tracing::debug;

pub struct CommandInspector<'cmd> {
    command: &'cmd Command,

    /// The entire command line as a list of arguments.
    /// The first argument is always the program/command to run.
    command_line: Vec<&'cmd OsString>,

    /// The entire input line to pass as stdin to a running command.
    input_line: Vec<&'cmd OsString>,
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
            command,
            command_line,
            input_line,
        }
    }

    pub fn get_command_line(&self) -> OsString {
        self.command_line
            .iter()
            .map(|a| a.to_os_string())
            .collect::<Vec<_>>()
            .join(OsStr::new(" "))
    }

    pub fn get_input_line(&self) -> Option<OsString> {
        if !self.should_pass_stdin() {
            return None;
        }

        let line = self
            .input_line
            .iter()
            .map(|i| i.to_os_string())
            .collect::<Vec<_>>()
            .join(OsStr::new(" "));

        Some(line)
    }

    pub fn get_prefix(&self) -> String {
        self.command.prefix.clone().unwrap_or_default()
    }

    pub fn should_error_nonzero(&self) -> bool {
        self.command.error_on_nonzero
    }

    pub fn should_pass_stdin(&self) -> bool {
        !self.command.input.is_empty() || self.should_pass_args_stdin()
    }

    pub fn should_pass_args_stdin(&self) -> bool {
        self.command
            .shell
            .as_ref()
            .map(|s| s.pass_args_stdin)
            .unwrap_or(false)
    }

    pub fn format_command(&self, line: &str) -> String {
        let workspace_root = PathBuf::from("."); // TODO
        let working_dir = self.command.cwd.as_ref().unwrap_or(&workspace_root);

        let target_dir = if working_dir == &workspace_root {
            String::from("workspace")
        } else {
            format!(
                ".{}{}",
                std::path::MAIN_SEPARATOR,
                working_dir
                    .strip_prefix(&workspace_root)
                    .unwrap()
                    .to_string_lossy(),
            )
        };

        format!(
            "{} {}",
            color::muted_light(line),
            color::muted(format!("(in {target_dir})"))
        )
    }

    pub fn log_command(&self) {
        let command_line = self.get_command_line();
        let input_line = self.get_input_line().unwrap_or_default();
        let line = if self.should_pass_args_stdin() {
            input_line.to_string_lossy()
        } else {
            command_line.to_string_lossy()
        };

        if self.command.print_command {
            println!("{}", self.format_command(&line));
        }

        // TODO
        // Avoid all this overhead if we're not logging
        // if !logging_enabled() {
        //     return;
        // }

        let debug_env = env::var("MOON_DEBUG_PROCESS_ENV").is_ok();
        let debug_input = env::var("MOON_DEBUG_PROCESS_INPUT").is_ok();

        let env_vars_field = self
            .command
            .env
            .iter()
            .filter(|(key, _)| {
                if debug_env {
                    true
                } else {
                    let key = key.to_str().unwrap_or_default();
                    key.starts_with("MOON_") || key.starts_with("PROTO_")
                }
            })
            .collect::<FxHashMap<_, _>>();

        let working_dir_field = self
            .command
            .cwd
            .as_ref()
            .map(|cwd| cwd.display().to_string());

        let format_input = |line: &str, input: &str| {
            format!(
                "{}{}{}",
                line,
                if line.ends_with('-') { " " } else { " - " },
                if input.len() > 200 && !debug_input {
                    "(truncated)".into()
                } else {
                    input.replace('\n', " ")
                }
            )
        };

        let debug_line = if let Some(shell) = &self.command.shell {
            let shell_line = format!("{} {}", shell.bin, shell.args.join(" "));

            if shell.pass_args_stdin {
                format_input(&shell_line, &line)
            } else {
                format!("{} {}", shell_line, line)
            }
        } else if !self.command.input.is_empty() {
            format_input(&line, input_line.to_str().unwrap())
        } else {
            line.to_string()
        };

        debug!(
            env_vars = ?env_vars_field,
            working_dir = working_dir_field,
            "Running command {}",
            color::shell(debug_line)
        );
    }
}
