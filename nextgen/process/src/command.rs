use crate::shell;
use moon_common::{color, is_ci, is_test_env};
use rustc_hash::FxHashMap;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Command {
    pub args: Vec<OsString>,

    pub bin: String,

    cwd: Option<PathBuf>,

    env: FxHashMap<OsString, OsString>,

    /// Convert non-zero exits to errors
    error_on_nonzero: bool,

    /// Values to pass to stdin
    input: Vec<OsString>,

    /// Prefix to prepend to all log lines
    prefix: Option<String>,

    /// Log the command to the terminal before running
    print_command: bool,

    /// Shell to wrap executing commands in
    shell: Option<shell::Shell>,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        let bin = bin.as_ref();

        let mut command = Command {
            bin: bin.to_string_lossy().to_string(),
            args: vec![],
            cwd: None,
            env: FxHashMap::default(),
            error_on_nonzero: true,
            input: vec![],
            prefix: None,
            print_command: false,
            shell: None,
        };

        // Referencing a batch script needs to be ran with a shell
        if shell::is_windows_script(&command.bin) {
            command.shell = Some(shell::create_shell());
        }

        command
    }

    pub fn arg<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Command {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn arg_if_missing<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Command {
        let arg = arg.as_ref();
        let present = self.args.iter().any(|a| a == arg);

        if !present {
            self.arg(arg);
        }

        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    pub fn cwd<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env
            .insert(key.as_ref().to_os_string(), val.as_ref().to_os_string());
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self.env(k, v);
        }

        self
    }

    pub fn inherit_colors(&mut self) -> &mut Command {
        let level = color::supports_color().to_string();

        self.env("FORCE_COLOR", &level);
        self.env("CLICOLOR_FORCE", &level);

        // Force a terminal width so that we have consistent sizing
        // in our cached output, and its the same across all machines
        // https://help.gnome.org/users/gnome-terminal/stable/app-terminal-sizes.html.en
        self.env("COLUMNS", "80");
        self.env("LINES", "24");

        self
    }

    pub fn input<I, V>(&mut self, input: I) -> &mut Command
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        for i in input {
            self.input.push(i.as_ref().to_os_string());
        }

        self
    }

    pub fn set_print_command(&mut self, state: bool) -> &mut Command {
        self.print_command = state;
        self
    }

    pub fn set_error_on_nonzero(&mut self, state: bool) -> &mut Command {
        self.error_on_nonzero = state;
        self
    }

    pub fn set_prefix(&mut self, prefix: &str, width: Option<usize>) -> &mut Command {
        if is_test_env() {
            self.prefix = Some(format!("[{prefix}]"));
        } else {
            self.prefix = Some(format!(
                "{} {} ",
                color::log_target(if let Some(width) = width {
                    format!("{: >width$}", prefix, width = width)
                } else {
                    prefix.to_owned()
                }),
                color::muted("|")
            ));
        }

        self
    }

    pub fn set_shell(&mut self, shell: shell::Shell) -> &mut Command {
        self.shell = Some(shell);
        self
    }

    pub fn with_shell(&mut self) -> &mut Command {
        self.set_shell(shell::create_shell());
        self
    }
}
