use cached::proc_macro::cached;
use moon_args::join_args_os;
use starbase_shell::BoxedShell;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub use starbase_shell::{ShellCommand, ShellType};

#[cached]
pub fn find_command_on_path(name: String) -> Option<PathBuf> {
    if name == "pwsh" || name == "powershell" {
        system_env::find_command_on_path("pwsh")
            .or_else(|| system_env::find_command_on_path("powershell"))
    } else {
        system_env::find_command_on_path(name)
    }
}

#[cached]
pub fn get_default_shell() -> ShellType {
    ShellType::detect_with_fallback()
}

#[inline]
pub fn is_windows_script<T: AsRef<OsStr>>(bin: T) -> bool {
    bin.as_ref()
        .to_str()
        .map(|bin| bin.to_lowercase())
        .is_some_and(|bin| bin.ends_with(".cmd") || bin.ends_with(".bat") || bin.ends_with(".ps1"))
}

#[derive(Debug)]
pub struct Shell {
    pub bin: PathBuf,
    pub bin_name: String,
    pub command: ShellCommand,
    pub instance: BoxedShell,
}

impl Shell {
    pub fn new(type_of: ShellType) -> Self {
        let bin_name = type_of.to_string();
        let instance = type_of.build();
        let command = instance.get_exec_command();

        Self {
            bin: find_command_on_path(bin_name.clone()).unwrap_or_else(|| bin_name.clone().into()),
            bin_name,
            command,
            instance,
        }
    }

    pub fn is_quoted(&self, arg: &str) -> bool {
        arg.starts_with("$'") || arg.starts_with("'") || arg.starts_with('"')
    }

    pub fn join_args(&self, args: Vec<OsString>) -> OsString {
        let mut line = OsString::new();
        let last_index = args.len() - 1;

        for (index, arg) in args.into_iter().enumerate() {
            let quoted_arg = match arg.to_str() {
                Some(inner) => {
                    if self.is_quoted(inner) {
                        arg
                    } else {
                        OsString::from(self.instance.quote(inner))
                    }
                }
                None => join_args_os([arg]),
            };

            line.push(&quoted_arg);

            if index != last_index {
                line.push(OsStr::new(" "));
            }
        }

        line
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new(get_default_shell())
    }
}
