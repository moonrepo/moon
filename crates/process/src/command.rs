// This implementation is loosely based on Cargo's:
// https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs

use crate::shell::Shell;
use moon_common::{color, is_test_env};
use moon_console::Console;
use rustc_hash::{FxHashMap, FxHasher};
use std::hash::Hasher;
use std::{
    ffi::{OsStr, OsString},
    sync::Arc,
};

pub struct Command {
    pub args: Vec<OsString>,

    pub bin: OsString,

    pub cwd: Option<OsString>,

    pub env: FxHashMap<OsString, Option<OsString>>,

    /// Convert non-zero exits to errors
    pub error_on_nonzero: bool,

    /// Escape/quote arguments when joining.
    pub escape_args: bool,

    /// Values to pass to stdin
    pub input: Vec<OsString>,

    /// Prefix to prepend to all log lines
    pub prefix: Option<String>,

    /// Log the command to the terminal before running
    pub print_command: bool,

    /// Shell to wrap executing commands in
    pub shell: Option<Shell>,

    /// Console to write output to
    pub console: Option<Arc<Console>>,

    /// Current ID of a running child process.
    pub current_id: Option<u32>,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        Command {
            bin: bin.as_ref().to_os_string(),
            args: vec![],
            cwd: None,
            env: FxHashMap::default(),
            error_on_nonzero: true,
            escape_args: true,
            input: vec![],
            prefix: None,
            print_command: false,
            shell: Some(Shell::default()),
            console: None,
            current_id: None,
        }
    }

    pub fn arg<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Self {
        self.args.push(arg.as_ref().to_os_string());
        self
    }

    pub fn arg_if_missing<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Self {
        let arg = arg.as_ref();
        let present = self.args.iter().any(|a| a == arg);

        if !present {
            self.arg(arg);
        }

        self
    }

    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    pub fn cwd<P: AsRef<OsStr>>(&mut self, dir: P) -> &mut Self {
        self.cwd = Some(dir.as_ref().to_os_string());
        self
    }

    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env.insert(
            key.as_ref().to_os_string(),
            Some(val.as_ref().to_os_string()),
        );
        self
    }

    pub fn env_if_missing<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref();
        if !self.env.contains_key(key) {
            self.env(key, val);
        }
        self
    }

    pub fn env_remove<K>(&mut self, key: K) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.env.insert(key.as_ref().to_os_string(), None);
        self
    }

    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Self
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

    pub fn inherit_colors(&mut self) -> &mut Self {
        let level = color::supports_color().to_string();

        if !is_test_env() {
            self.env_remove("NO_COLOR");
            self.env("FORCE_COLOR", &level);
            self.env("CLICOLOR_FORCE", &level);
        }

        // Force a terminal width so that we have consistent sizing
        // in our cached output, and its the same across all machines
        // https://help.gnome.org/users/gnome-terminal/stable/app-terminal-sizes.html.en
        self.env("COLUMNS", "80");
        self.env("LINES", "24");

        self
    }

    pub fn input<I, V>(&mut self, input: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        for i in input {
            self.input.push(i.as_ref().to_os_string());
        }

        self
    }

    pub fn get_bin_name(&self) -> String {
        self.bin.to_string_lossy().to_string()
    }

    pub fn get_cache_key(&self) -> String {
        let mut hasher = FxHasher::default();

        let mut write = |value: &OsString| {
            hasher.write(value.as_os_str().as_encoded_bytes());
        };

        for (key, value) in &self.env {
            if let Some(value) = value {
                write(key);
                write(value);
            }
        }

        write(&self.bin);

        for arg in &self.args {
            write(arg);
        }

        if let Some(cwd) = &self.cwd {
            write(cwd);
        }

        for arg in &self.input {
            write(arg);
        }

        format!("{}", hasher.finish())
    }

    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub fn set_print_command(&mut self, state: bool) -> &mut Self {
        self.print_command = state;
        self
    }

    pub fn set_error_on_nonzero(&mut self, state: bool) -> &mut Self {
        self.error_on_nonzero = state;
        self
    }

    pub fn set_prefix(&mut self, prefix: &str) -> &mut Self {
        self.prefix = Some(prefix.to_owned());
        self
    }

    pub fn should_error_nonzero(&self) -> bool {
        self.error_on_nonzero
    }

    pub fn should_pass_stdin(&self) -> bool {
        !self.input.is_empty() || self.should_pass_args_stdin()
    }

    pub fn should_pass_args_stdin(&self) -> bool {
        self.shell
            .as_ref()
            .map(|shell| shell.command.pass_args_stdin)
            .unwrap_or(false)
    }

    pub fn with_console(&mut self, console: Arc<Console>) -> &mut Self {
        self.console = Some(console);
        self
    }

    pub fn with_shell(&mut self, shell: Shell) -> &mut Self {
        self.shell = Some(shell);
        self
    }

    pub fn without_shell(&mut self) -> &mut Self {
        self.shell = None;
        self
    }
}
