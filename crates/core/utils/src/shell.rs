use cached::proc_macro::cached;
use std::env;
use tokio::process::Command as TokioCommand;

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

// https://thinkpowershell.com/decision-to-switch-to-powershell-core-pwsh/
#[inline]
pub fn create_windows_shell() -> (String, TokioCommand) {
    let shell = if is_program_on_path("pwsh.exe".into()) {
        "pwsh.exe".into()
    } else {
        "powershell.exe".into()
    };

    let mut cmd = TokioCommand::new(&shell);
    cmd.arg("-NonInteractive");
    cmd.arg("-NoLogo");
    cmd.arg("-NoProfile");

    // We'll pass the command args via stdin, so that paths with special
    // characters and spaces resolve correctly.
    // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.2#-command---c
    cmd.arg("-Command");
    cmd.arg("-");

    (shell, cmd)
}
