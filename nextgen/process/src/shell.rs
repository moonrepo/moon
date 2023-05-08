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
    pub bin: String,
    pub args: Vec<String>,
    pub pass_args_stdin: bool,
}

// https://thinkpowershell.com/decision-to-switch-to-powershell-core-pwsh/
#[cfg(windows)]
#[inline]
pub fn create_shell(with_profile: bool) -> Shell {
    let mut args: Vec<String> = vec!["-NonInteractive".into(), "-NoLogo".into()];

    if !with_profile {
        args.push("-NoProfile".into());
    }

    args.push("-Command".into());

    // We'll pass the command args via stdin, so that paths with special
    // characters and spaces resolve correctly.
    // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.2#-command---c
    args.push("-".into());

    Shell {
        bin: if is_program_on_path("pwsh.exe".into()) {
            "pwsh.exe".into()
        } else {
            "powershell.exe".into()
        },
        args,
        pass_args_stdin: true,
    }
}

#[cfg(not(windows))]
#[inline]
pub fn create_shell() -> Shell {
    Shell {
        bin: env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
        args: vec!["-c".into()],
        pass_args_stdin: false,
    }
}
