// This implementation is loosely based on Cargo's:
// https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/process_builder.rs

use crate::shell::Shell;
use moon_common::{color, is_test_env};
use moon_console::Console;
use moon_env_var::GlobalEnvBag;
use rustc_hash::{FxHashMap, FxHasher};
use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::hash::Hasher;
use std::sync::Arc;

#[derive(Debug)]
pub enum CommandExecutable {
    /// Single file name: git
    Binary(OsString),
    /// Full script: git commit -m ""
    Script(OsString),
}

impl CommandExecutable {
    pub fn as_os_str(&self) -> &OsStr {
        match &self {
            Self::Binary(inner) => inner,
            Self::Script(inner) => inner,
        }
    }

    pub fn into_os_string(self) -> OsString {
        match self {
            Self::Binary(inner) => inner,
            Self::Script(inner) => inner,
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub args: VecDeque<OsString>,

    /// Continuously write to stdin and read from stdout
    pub continuous_pipe: bool,

    pub cwd: Option<OsString>,

    pub env: FxHashMap<OsString, Option<OsString>>,

    pub exe: CommandExecutable,

    /// Convert non-zero exits to errors
    pub error_on_nonzero: bool,

    /// Escape/quote arguments when joining
    pub escape_args: bool,

    /// Values to pass to stdin
    pub input: Vec<OsString>,

    /// Paths to prepend to `PATH`
    pub paths: VecDeque<OsString>,

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
    pub fn new<T: AsRef<OsStr>>(bin: T) -> Self {
        Command {
            args: VecDeque::new(),
            continuous_pipe: false,
            cwd: None,
            env: FxHashMap::default(),
            exe: CommandExecutable::Binary(bin.as_ref().to_os_string()),
            error_on_nonzero: true,
            escape_args: true,
            input: vec![],
            paths: VecDeque::new(),
            prefix: None,
            print_command: false,
            shell: Some(Shell::default()),
            console: None,
        }
    }

    pub fn new_script<T: AsRef<OsStr>>(script: T) -> Self {
        let mut command = Self::new(script);
        command.exe = CommandExecutable::Script(command.exe.into_os_string());
        command
    }

    pub fn arg<A: AsRef<OsStr>>(&mut self, arg: A) -> &mut Self {
        self.args.push_back(arg.as_ref().to_os_string());
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

    pub fn envs_remove<I, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        for v in vars {
            self.env_remove(v);
        }

        self
    }

    pub fn envs_if_missing<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self.env_if_missing(k, v);
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
        for path in list {
            self.paths.push_back(path.as_ref().to_os_string());
        }

        self
    }

    pub fn prepend_paths<I, V>(&mut self, list: I) -> &mut Self
    where
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        let mut paths = vec![];

        for path in list {
            paths.push(path.as_ref().to_os_string());
        }

        for path in paths.into_iter().rev() {
            self.paths.push_front(path);
        }

        self
    }

    pub fn get_args_list(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect()
    }

    pub fn get_bin_name(&self) -> String {
        match &self.exe {
            CommandExecutable::Binary(bin) => bin.to_string_lossy().to_string(),
            CommandExecutable::Script(script) => {
                if let Some(inner) = script.to_str() {
                    match inner.find(' ') {
                        Some(index) => &inner[0..index],
                        None => inner,
                    }
                    .into()
                } else {
                    let mut bytes = vec![];

                    for ch in script.as_encoded_bytes() {
                        if *ch == b' ' {
                            break;
                        }

                        bytes.push(*ch);
                    }

                    unsafe { OsString::from_encoded_bytes_unchecked(bytes) }
                        .to_string_lossy()
                        .to_string()
                }
            }
        }
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

        match &self.exe {
            CommandExecutable::Binary(exe) => {
                write(exe);
            }
            CommandExecutable::Script(exe) => {
                write(exe);
            }
        };

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

    pub fn get_script(&self) -> String {
        match &self.exe {
            CommandExecutable::Binary(_) => String::new(),
            CommandExecutable::Script(script) => script.to_string_lossy().to_string(),
        }
    }

    pub fn no_shell(&mut self) -> &mut Self {
        self.shell = None;
        self
    }

    pub fn set_bin<T: AsRef<OsStr>>(&mut self, bin: T) -> &mut Self {
        self.exe = CommandExecutable::Binary(bin.as_ref().to_os_string());
        self
    }

    pub fn set_console(&mut self, console: Arc<Console>) -> &mut Self {
        self.console = Some(console);
        self
    }

    pub fn set_continuous_pipe(&mut self, state: bool) -> &mut Self {
        self.continuous_pipe = state;
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

    pub fn set_print_command(&mut self, state: bool) -> &mut Self {
        self.print_command = state;
        self
    }

    pub fn set_script<T: AsRef<OsStr>>(&mut self, script: T) -> &mut Self {
        self.exe = CommandExecutable::Script(script.as_ref().to_os_string());
        self
    }

    pub fn set_shell(&mut self, shell: Shell) -> &mut Self {
        self.shell = Some(shell);
        self
    }

    pub fn should_error_nonzero(&self) -> bool {
        self.error_on_nonzero
    }

    pub fn should_pass_args_stdin(&self) -> bool {
        self.shell
            .as_ref()
            .map(|shell| shell.command.pass_args_stdin)
            .unwrap_or(false)
    }

    pub fn should_pass_stdin(&self) -> bool {
        !self.input.is_empty() || self.should_pass_args_stdin()
    }
}
