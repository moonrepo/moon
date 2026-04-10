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
