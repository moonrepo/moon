use cached::proc_macro::cached;
use std::env::consts;
use std::ffi::OsStr;
use std::path::PathBuf;

pub use starbase_shell::{ShellCommand, ShellType};

#[cached]
fn find_command_on_path(name: String) -> Option<PathBuf> {
    system_env::find_command_on_path(name)
}

#[cached]
fn get_default_shell() -> ShellType {
    ShellType::detect().unwrap_or_else(|| {
        if consts::OS == "windows" {
            ShellType::Pwsh
        } else {
            ShellType::Bash
        }
    })
}

#[inline]
pub fn is_windows_script<T: AsRef<OsStr>>(bin: T) -> bool {
    let bin = bin.as_ref().to_string_lossy();

    bin.ends_with(".cmd")
        || bin.ends_with(".bat")
        || bin.ends_with(".ps1")
        || bin.ends_with(".CMD")
        || bin.ends_with(".BAT")
        || bin.ends_with(".PS1")
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
