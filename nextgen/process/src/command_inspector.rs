use crate::command::Command;
use moon_args::join_args_os;
use moon_common::color;
use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use tracing::{debug, enabled};

type LineValue<'l> = Cow<'l, OsStr>;

#[derive(Debug)]
pub struct CommandLine<'l> {
    pub command: Vec<LineValue<'l>>,
    pub input: LineValue<'l>,
    pub main_command: LineValue<'l>,
}

impl<'l> CommandLine<'l> {
    pub fn new(command: &Command) -> CommandLine {
        let mut command_line: Vec<LineValue> = vec![];
        let mut input_line: Vec<LineValue> = vec![];
        let mut main_line: Vec<LineValue> = vec![];
        // let mut join_input = false;

        let push_to_line = |line: &mut Vec<LineValue>| {
            line.push(Cow::Owned(command.bin.to_owned()));

            for arg in &command.args {
                line.push(Cow::Owned(arg.to_owned()));
            }
        };

        // Extract the main command, without shell, for other purposes!
        push_to_line(&mut main_line);

        // If wrapped in a shell, the shell binary and arguments
        // must be placed at the start of the line.
        if let Some(shell) = &command.shell {
            command_line.push(Cow::Borrowed(shell.bin.as_os_str()));
            command_line.extend(shell.args.iter().map(|arg| Cow::Borrowed(OsStr::new(arg))));

            // If the main command should be passed via stdin,
            // then append the input line instead of the command line.
            if shell.pass_args_stdin {
                // join_input = true;
                push_to_line(&mut input_line);

                // Otherwise append as a *single* argument. This typically
                // appears after a "-" argument (should come from shell).
            } else {
                let mut sub_line: Vec<LineValue> = vec![];
                push_to_line(&mut sub_line);

                command_line.push(Cow::Owned(join_args_os(sub_line)));
            }

            // Otherwise we have a normal command and arguments.
        } else {
            push_to_line(&mut command_line);

            // That also may have input.
            if !command.input.is_empty() {
                for input in &command.input {
                    input_line.push(Cow::Borrowed(input));
                }
            }
        }

        CommandLine {
            command: command_line,
            // input: if join_input {
            //     join_args(input_line)
            // } else {
            //     input_line.join("")
            // },
            input: Cow::Owned(input_line.join(OsStr::new(" "))),
            main_command: Cow::Owned(join_args_os(main_line)),
        }
    }
}

impl<'l> Display for CommandLine<'l> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let command = join_args_os(&self.command);
        let command = command.to_string_lossy();

        write!(f, "{}", command)?;

        if !self.input.is_empty() {
            let debug_input = env::var("MOON_DEBUG_PROCESS_INPUT").is_ok();
            let input = &self.input;

            if !command.ends_with('-') {
                write!(f, " -")?;
            }

            write!(
                f,
                " {}",
                if input.len() > 200 && !debug_input {
                    "(truncated)".into()
                } else {
                    input.to_string_lossy().trim().replace('\n', " ")
                }
            )?;
        }

        Ok(())
    }
}

pub struct CommandInspector<'cmd> {
    command: &'cmd Command,
    line_cache: OnceCell<CommandLine<'cmd>>,
}

impl<'cmd> CommandInspector<'cmd> {
    pub fn new(command: &'cmd Command) -> Self {
        Self {
            command,
            line_cache: OnceCell::new(),
        }
    }

    pub fn get_cache_key(&self) -> String {
        let line = self.get_command_line();

        format!(
            "{}{}",
            line.command.join(OsStr::new(" ")).to_string_lossy(),
            line.input.to_string_lossy()
        )
    }

    pub fn get_command_line(&self) -> &CommandLine {
        self.line_cache
            .get_or_init(|| CommandLine::new(self.command))
    }

    pub fn get_prefix(&self) -> String {
        self.command.prefix.clone().unwrap_or_default()
    }

    pub fn get_workspace_root(&self) -> PathBuf {
        let env_key = OsString::from("MOON_WORKSPACE_ROOT");

        env::var_os(&env_key)
            .or_else(|| self.command.env.get(&env_key).cloned())
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or(PathBuf::from(".")))
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

    pub fn format_command(
        &self,
        line: &str,
        workspace_root: &Path,
        working_dir: Option<&Path>,
    ) -> String {
        let working_dir = working_dir.unwrap_or(workspace_root);

        let target_dir = if working_dir == workspace_root {
            "workspace".into()
        } else if let Ok(dir) = working_dir.strip_prefix(workspace_root) {
            format!(".{}{}", MAIN_SEPARATOR, dir.to_string_lossy())
        } else {
            debug!(
                working_dir = ?working_dir,
                workspace_root = ?workspace_root,
                "Unable to determine the directory a task is running in...",
            );

            ".".into()
        };

        format!(
            "{} {}",
            color::muted_light(line.trim()),
            color::muted(format!("(in {target_dir})"))
        )
    }

    pub fn log_command(&self) {
        let mut workspace_root = None;

        if self.command.print_command {
            if let Some(cmd_line) = self.get_command_line().main_command.to_str() {
                if let Some(console) = self.command.console.as_ref() {
                    if !console.out.is_quiet() {
                        workspace_root = Some(self.get_workspace_root());

                        let cmd_line = self.format_command(
                            cmd_line,
                            workspace_root.as_deref().unwrap(),
                            self.command.cwd.as_deref(),
                        );

                        let _ = console.out.write_line(cmd_line);
                    }
                }
            }
        }

        // Avoid all this overhead if we're not logging
        if !enabled!(tracing::Level::DEBUG) {
            return;
        }

        let debug_env = env::var("MOON_DEBUG_PROCESS_ENV").is_ok();

        let env_vars_field = self
            .command
            .env
            .iter()
            .filter(|(key, _)| {
                if debug_env {
                    true
                } else {
                    key.to_str()
                        .map(|k| k.starts_with("MOON_"))
                        .unwrap_or_default()
                }
            })
            .collect::<FxHashMap<_, _>>();

        if workspace_root.is_none() {
            workspace_root = Some(self.get_workspace_root());
        }

        let working_dir_field = self
            .command
            .cwd
            .as_deref()
            .or_else(|| workspace_root.as_deref());

        debug!(
            env_vars = ?env_vars_field,
            working_dir = ?working_dir_field,
            "Running command {}",
            color::shell(self.get_command_line().to_string())
        );
    }
}
