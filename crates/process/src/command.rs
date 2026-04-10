use crate::helpers::get_default_shell;
use moon_common::{color, is_test_env};
use moon_console::Console;
use moon_env_var::GlobalEnvBag;
use rustc_hash::{FxHashMap, FxHasher};
use starbase_shell::{ShellType, join_exe_args};
use std::collections::VecDeque;
use std::ffi::{OsStr, OsString};
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, PartialEq)]
pub enum Env {
    /// Always set and overwrite system var
    Set(OsString),

    /// Only set if system var is not set
    SetIfMissing(OsString),

    /// Unset system var and don't inherit
    Unset,
}

impl Env {
    pub fn get_value(&self) -> Option<&OsString> {
        match self {
            Env::Set(value) => Some(value),
            Env::SetIfMissing(value) => Some(value),
            Env::Unset => None,
        }
    }
}

#[derive(Debug)]
pub struct CommandArg {
    // In shells: "value"
    pub quoted_value: Option<OsString>,

    // Not in shells: value
    pub value: OsString,
}

impl CommandArg {
    pub fn as_os_str(&self) -> &OsStr {
        self.quoted_value.as_ref().unwrap_or(&self.value)
    }
}

impl AsRef<OsStr> for CommandArg {
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl From<&str> for CommandArg {
    fn from(value: &str) -> Self {
        Self::from(OsString::from(value))
    }
}

impl From<&String> for CommandArg {
    fn from(value: &String) -> Self {
        Self::from(OsString::from(value))
    }
}

impl From<String> for CommandArg {
    fn from(value: String) -> Self {
        Self::from(OsString::from(value))
    }
}

impl From<&OsStr> for CommandArg {
    fn from(value: &OsStr) -> Self {
        Self::from(value.to_os_string())
    }
}

impl From<&OsString> for CommandArg {
    fn from(value: &OsString) -> Self {
        Self::from(value.to_os_string())
    }
}

impl From<OsString> for CommandArg {
    fn from(value: OsString) -> Self {
        Self {
            quoted_value: None,
            value,
        }
    }
}

impl From<&Path> for CommandArg {
    fn from(value: &Path) -> Self {
        Self::from(value.as_os_str())
    }
}

impl From<&PathBuf> for CommandArg {
    fn from(value: &PathBuf) -> Self {
        Self::from(value.as_os_str())
    }
}

impl From<PathBuf> for CommandArg {
    fn from(value: PathBuf) -> Self {
        Self::from(value.into_os_string())
    }
}

#[derive(Debug)]
pub enum CommandExecutable {
    /// Single file name: git
    Binary(CommandArg),

    /// Full script: git commit --allow-empty
    Script(OsString),
}

impl CommandExecutable {
    pub fn as_os_str(&self) -> &OsStr {
        match self {
            Self::Binary(inner) => &inner.value,
            Self::Script(inner) => inner,
        }
    }

    pub fn requires_shell(&self) -> bool {
        match self {
            Self::Binary(_) => false,
            Self::Script(_) => true,
        }
    }
}

#[derive(Debug)]
pub struct Command {
    pub args: VecDeque<CommandArg>,

    /// Continuously write to stdin and read from stdout
    pub continuous_pipe: bool,

    pub cwd: Option<OsString>,

    pub env: FxHashMap<OsString, Env>,

    pub exe: CommandExecutable,

    /// Convert non-zero exits to errors
    pub error_on_nonzero: bool,

    /// Values to pass to stdin
    pub input: Vec<OsString>,

    /// Paths to prepend to `PATH`
    pub paths: VecDeque<OsString>,

    /// Prefix to prepend to all log lines
    pub prefix: Option<String>,

    /// Log the command to the terminal before running
    pub print_command: bool,

    /// Shell to wrap executing commands in
    pub shell: Option<ShellType>,

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
            exe: CommandExecutable::Binary(CommandArg {
                quoted_value: None,
                value: bin.as_ref().to_os_string(),
            }),
            error_on_nonzero: true,
            input: vec![],
            paths: VecDeque::new(),
            prefix: None,
            print_command: false,
            shell: Some(get_default_shell()),
            console: None,
        }
    }

    pub fn new_bin<T: Into<CommandArg>>(bin: T) -> Self {
        let mut command = Self::new("");
        command.exe = CommandExecutable::Binary(bin.into());
        command
    }

    pub fn new_script<T: AsRef<OsStr>>(script: T) -> Self {
        let mut command = Self::new("");
        command.exe = CommandExecutable::Script(script.as_ref().to_os_string());
        command
    }

    pub fn arg<A: Into<CommandArg>>(&mut self, arg: A) -> &mut Self {
        self.args.push_back(arg.into());
        self
    }

    pub fn arg_if_missing<A: Into<CommandArg>>(&mut self, arg: A) -> &mut Self {
        let arg = arg.into();

        if !self.contains_arg(&arg.value) {
            self.arg(arg);
        }

        self
    }

    pub fn args<I, A>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = A>,
        A: Into<CommandArg>,
    {
        for arg in args {
            self.arg(arg);
        }

        self
    }

    pub fn contains_arg<A>(&self, arg: A) -> bool
    where
        A: AsRef<OsStr>,
    {
        let arg = arg.as_ref();
        self.args
            .iter()
            .any(|a| a.value == arg || a.quoted_value.as_ref().is_some_and(|aa| aa == arg))
    }

    pub fn contains_env<K>(&self, key: K) -> bool
    where
        K: AsRef<OsStr>,
    {
        self.env.contains_key(key.as_ref())
    }

    pub fn cwd<P: AsRef<OsStr>>(&mut self, dir: P) -> &mut Self {
        self.cwd = Some(dir.as_ref().to_os_string());
        self
    }

    pub fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env_opt(key, Some(value))
    }

    pub fn env_opt<K, V>(&mut self, key: K, value: Option<V>) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.env_with_behavior(
            key,
            match value {
                Some(v) => Env::Set(v.as_ref().to_os_string()),
                None => Env::Unset,
            },
        )
    }

    pub fn env_remove<K>(&mut self, key: K) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.env_with_behavior(key, Env::Unset)
    }

    pub fn env_with_behavior<K>(&mut self, key: K, value: Env) -> &mut Self
    where
        K: AsRef<OsStr>,
    {
        self.env.insert(key.as_ref().to_os_string(), value);
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

    pub fn envs_opt<I, K, V>(&mut self, vars: I) -> &mut Self
    where
        I: IntoIterator<Item = (K, Option<V>)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        for (k, v) in vars {
            self.env_opt(k, v);
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

    pub fn inherit_colors(&mut self) -> &mut Self {
        let bag = GlobalEnvBag::instance();

        // Don't show colors in our own tests, as it disrupts snapshots,
        // and only inherit colors if the current command hasn't
        // explicitly configured these variables
        if !is_test_env()
            && !self.contains_env("NO_COLOR")
            && !self.contains_env("FORCE_COLOR")
            && !bag.has("NO_COLOR")
            && !bag.has("FORCE_COLOR")
        {
            let level = color::supports_color().to_string();

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
            .map(|arg| arg.value.to_string_lossy().to_string())
            .collect()
    }

    pub fn get_bin_name(&self) -> String {
        match &self.exe {
            CommandExecutable::Binary(bin) => bin.value.to_string_lossy().to_string(),
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
            write(key);

            match value {
                Env::Set(value) => write(value),
                Env::SetIfMissing(value) => write(value),
                Env::Unset => {}
            };
        }

        match &self.exe {
            CommandExecutable::Binary(exe) => {
                write(&exe.value);
            }
            CommandExecutable::Script(exe) => {
                write(exe);
            }
        };

        for arg in &self.args {
            write(&arg.value);
        }

        if let Some(cwd) = &self.cwd {
            write(cwd);
        }

        for arg in &self.input {
            write(arg);
        }

        format!("{}", hasher.finish())
    }

    pub fn get_command_line(&self, with_shell: bool, with_input: bool) -> String {
        let shell = self.shell.unwrap_or_default().build();
        let use_shell = with_shell && (self.shell.is_some() || self.exe.requires_shell());
        let mut line = OsString::new();

        if use_shell {
            line.push(shell.to_string());
            line.push(" -c “");
        }

        match &self.exe {
            CommandExecutable::Binary(bin) => {
                line.push(join_exe_args(&shell, bin, &self.args, false));
            }
            CommandExecutable::Script(script) => {
                line.push(script);
            }
        };

        if use_shell {
            line.push("”");
        }

        if with_input && !self.input.is_empty() {
            let debug_input = GlobalEnvBag::instance().should_debug_process_input();
            let input = self.input.join(OsStr::new(" "));

            line.push(" - ");

            if input.len() > 200 && !debug_input {
                line.push(format!(
                    "(truncated input, {} total bytes)",
                    self.get_input_size()
                ));
            } else {
                line.push(input);
            }
        }

        line.to_string_lossy().trim().replace('\n', " ")
    }

    pub fn get_input_size(&self) -> usize {
        self.input.iter().map(|i| i.len()).sum()
    }

    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub fn get_script(&self) -> String {
        match &self.exe {
            CommandExecutable::Binary(bin) => bin.value.to_string_lossy().to_string(),
            CommandExecutable::Script(script) => script.to_string_lossy().to_string(),
        }
    }

    pub fn no_shell(&mut self) -> &mut Self {
        self.shell = None;
        self
    }

    pub fn set_bin<T: Into<CommandArg>>(&mut self, bin: T) -> &mut Self {
        self.exe = CommandExecutable::Binary(bin.into());
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
        if self.shell.is_none() {
            self.shell = Some(get_default_shell());
        }

        self.exe = CommandExecutable::Script(script.as_ref().to_os_string());
        self
    }

    pub fn set_shell(&mut self, shell: ShellType) -> &mut Self {
        self.shell = Some(shell);
        self
    }

    pub fn should_error_nonzero(&self) -> bool {
        self.error_on_nonzero
    }

    pub fn should_pass_stdin(&self) -> bool {
        !self.input.is_empty()
    }
}
