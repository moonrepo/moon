// This implementation is loosely based on Cargo's:
// https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs

use crate::shell::Shell;
use miette::IntoDiagnostic;
use moon_common::{color, is_test_env};
use moon_console::Console;
use moon_env_var::GlobalEnvBag;
use rustc_hash::{FxHashMap, FxHasher};
use std::env;
use std::ffi::{OsStr, OsString};
use std::hash::Hasher;
use std::sync::Arc;

#[derive(Debug)]
pub struct Command {
    pub args: Vec<OsString>,

    pub bin: OsString,

    /// Continuously write to stdin and read from stdout.
    pub continuous_pipe: bool,

    pub cwd: Option<OsString>,

    pub env: FxHashMap<OsString, Option<OsString>>,

    /// Convert non-zero exits to errors
    pub error_on_nonzero: bool,

    /// Escape/quote arguments when joining.
    pub escape_args: bool,

    /// Values to pass to stdin
    pub input: Vec<OsString>,

    /// Paths to append to `PATH`
    pub paths_after: Vec<OsString>,

    /// Paths to prepend to `PATH`
    pub paths_before: Vec<OsString>,

    /// Prefix to prepend to all log lines
    pub prefix: Option<String>,

    /// Log the command to the terminal before running
    pub print_command: bool,

    /// Shell to wrap executing commands in
    pub shell: Option<Shell>,

    /// Console to write output to
    pub console: Option<Arc<Console>>,
}

impl Command {
    pub fn new<S: AsRef<OsStr>>(bin: S) -> Self {
        Command {
            bin: bin.as_ref().to_os_string(),
            args: vec![],
            continuous_pipe: false,
            cwd: None,
            env: FxHashMap::default(),
            error_on_nonzero: true,
            escape_args: true,
            input: vec![],
            paths_after: vec![],
            paths_before: vec![],
            prefix: None,
            print_command: false,
            shell: Some(Shell::default()),
            console: None,
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
        let dir = dir.as_ref().to_os_string();

        self.env("PWD", &dir);
        self.cwd = Some(dir);
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

    pub fn envs_if_not_global<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let bag = GlobalEnvBag::instance();

        for (k, v) in vars {
            let k = k.as_ref();

            if !bag.has(k) {
                self.env(k, v);
            }
        }

        self
    }

    pub fn inherit_colors(&mut self) -> &mut Self {
        // Don't show colors in our own tests, as it disrupts snapshots
        if !is_test_env() {
            let no_color = OsString::from("NO_COLOR");
            let force_color = OsString::from("FORCE_COLOR");

            // Only inherit colors if the current command hasn't
            // explicitly configured these variables
            if !self.env.contains_key(&no_color) && !self.env.contains_key(&force_color) {
                let level = color::supports_color().to_string();

                self.env_remove(no_color);
                self.env(force_color, &level);
                self.env("CLICOLOR_FORCE", &level);
            }
        }

        // Force a terminal width so that we have consistent sizing
        // in our cached output, and its the same across all machines
        // https://help.gnome.org/users/gnome-terminal/stable/app-terminal-sizes.html.en
        self.env("COLUMNS", "80");
        self.env("LINES", "24");

        self
    }

    pub fn inherit_path(&mut self) -> miette::Result<&mut Self> {
        let key = OsString::from("PATH");

        if self.env.contains_key(&key)
            || (self.paths_before.is_empty() && self.paths_after.is_empty())
        {
            return Ok(self);
        }

        let mut paths = vec![];

        paths.extend(self.paths_before.clone());

        for path in env::split_paths(&env::var_os(&key).unwrap_or_default()) {
            paths.push(path.into_os_string());
        }

        paths.extend(self.paths_after.clone());

        self.env(&key, env::join_paths(paths).into_diagnostic()?);

        Ok(self)
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

    pub fn append_paths<I, V>(&mut self, list: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        self.paths_after
            .extend(list.into_iter().map(|path| path.as_ref().to_os_string()));

        self
    }

    pub fn prepend_paths<I, V>(&mut self, list: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        let mut list = list
            .into_iter()
            .map(|path| path.as_ref().to_os_string())
            .collect::<Vec<_>>();

        list.extend(std::mem::take(&mut self.paths_before));

        self.paths_before = list;
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

    pub fn set_continuous_pipe(&mut self, state: bool) -> &mut Self {
        self.continuous_pipe = state;
        self
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
