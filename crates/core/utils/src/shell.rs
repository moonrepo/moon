use crate::string_vec;
use cached::proc_macro::cached;
use std::env;

#[cached]
#[inline]
fn is_program_on_path(program_name: String) -> bool {
    let Some(system_path) = env::var_os("PATH") else {
        return false;
    };

    for path_dir in env::split_paths(&system_path) {
        #[allow(clippy::needless_borrow)]
        if path_dir.join(&program_name).exists() {
            return true;
        }
    }

    false
}

#[derive(Debug)]
pub struct Shell {
    pub command: String,
    pub args: Vec<String>,
}

// https://thinkpowershell.com/decision-to-switch-to-powershell-core-pwsh/
#[inline]
pub fn create_windows_shell() -> Shell {
    Shell {
        command: if is_program_on_path("pwsh.exe".into()) {
            "pwsh.exe".into()
        } else {
            "powershell.exe".into()
        },
        args: string_vec![
            "-NonInteractive",
            "-NoLogo",
            "-NoProfile",
            "-Command",
            // We'll pass the command args via stdin, so that paths with special
            // characters and spaces resolve correctly.
            // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.2#-command---c
            "-"
        ],
    }
}
