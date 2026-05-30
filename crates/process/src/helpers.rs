use moon_common::color;
use starbase_shell::ShellType;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub fn find_command_on_path(name: &str) -> Option<PathBuf> {
    if name == "pwsh" || name == "powershell" {
        system_env::find_command_on_path("pwsh")
            .or_else(|| system_env::find_command_on_path("powershell"))
    } else {
        system_env::find_command_on_path(name)
    }
}

pub fn get_default_shell() -> ShellType {
    static SHELL_CACHE: OnceLock<ShellType> = OnceLock::new();

    *SHELL_CACHE.get_or_init(ShellType::detect_with_fallback)
}

pub fn format_command_line(command: &str, workspace_root: &Path, working_dir: &Path) -> String {
    let dir = if working_dir == workspace_root {
        "workspace".into()
    } else if let Ok(dir) = working_dir.strip_prefix(workspace_root) {
        format!(".{}{}", std::path::MAIN_SEPARATOR, dir.to_string_lossy())
    } else {
        ".".into()
    };

    format!(
        "{} {}",
        color::muted_light(command.trim()),
        color::muted(format!("(in {dir})"))
    )
}
