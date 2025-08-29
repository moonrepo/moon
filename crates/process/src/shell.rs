use cached::proc_macro::cached;
use starbase_shell::join_args;
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

pub use starbase_shell::{BoxedShell, ShellCommand, ShellType};

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

    pub fn join_args(&self, args: Vec<OsString>) -> OsString {
        OsString::from(join_args(
            &self.instance,
            args.iter().filter_map(|arg| arg.to_str()),
        ))
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new(get_default_shell())
    }
}
