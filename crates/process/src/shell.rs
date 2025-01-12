use cached::proc_macro::cached;
use std::ffi::OsStr;
use std::path::PathBuf;

pub use starbase_shell::{ShellCommand, ShellType};

#[cached]
fn find_command_on_path(name: String) -> Option<PathBuf> {
    if name == "pwsh" || name == "powershell" {
        system_env::find_command_on_path("pwsh")
            .or_else(|| system_env::find_command_on_path("powershell"))
    } else {
        system_env::find_command_on_path(name)
    }
}

#[cached]
fn get_default_shell() -> ShellType {
    ShellType::detect_with_fallback()
}

#[inline]
pub fn is_windows_script<T: AsRef<OsStr>>(bin: T) -> bool {
    bin.as_ref().to_str().is_some_and(|bin| {
        bin.ends_with(".cmd")
            || bin.ends_with(".bat")
            || bin.ends_with(".ps1")
            || bin.ends_with(".CMD")
            || bin.ends_with(".BAT")
            || bin.ends_with(".PS1")
    })
}

pub struct Shell {
    pub bin: PathBuf,
    pub command: ShellCommand,
}

impl Shell {
    pub fn new(type_of: ShellType) -> Self {
        let bin_name = type_of.to_string();
        let command = type_of.build().get_exec_command();

        Self {
            bin: find_command_on_path(bin_name.clone()).unwrap_or_else(|| bin_name.into()),
            command,
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new(get_default_shell())
    }
}
