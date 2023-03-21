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
    pub pass_args_stdin: bool,
}

// https://thinkpowershell.com/decision-to-switch-to-powershell-core-pwsh/
#[inline]
pub fn create_windows_shell(with_profile: bool) -> Shell {
    let mut args = string_vec!["-NonInteractive", "-NoLogo",];

    if !with_profile {
        args.push("-NoProfile".into());
    }

    args.push("-Command".into());

    // We'll pass the command args via stdin, so that paths with special
    // characters and spaces resolve correctly.
    // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.2#-command---c
    args.push("-".into());

    Shell {
        command: if is_program_on_path("pwsh.exe".into()) {
            "pwsh.exe".into()
        } else {
            "powershell.exe".into()
        },
        args,
        pass_args_stdin: true,
    }
}

#[inline]
pub fn create_unix_shell() -> Shell {
    Shell {
        command: env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
        args: string_vec!["-c"],
        pass_args_stdin: false,
    }
}
