use cached::proc_macro::cached;
use std::path::PathBuf;

pub use starbase_shell::{BoxedShell, ShellType};

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

#[derive(Debug)]
pub struct Shell {
    pub bin: PathBuf,
    pub bin_name: String,
    pub instance: BoxedShell,
}

impl Shell {
    pub fn new(type_of: ShellType) -> Self {
        let bin_name = type_of.to_string();
        let instance = type_of.build();

        Self {
            bin: find_command_on_path(bin_name.clone()).unwrap_or_else(|| bin_name.clone().into()),
            bin_name,
            instance,
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new(get_default_shell())
    }
}
