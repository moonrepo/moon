use cached::proc_macro::cached;
use std::env::{self, consts};
use std::ffi::OsStr;
use std::path::PathBuf;

#[cached]
#[inline]
fn find_command_on_path(name: String) -> Option<PathBuf> {
    system_env::find_command_on_path(name)
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

#[derive(Debug)]
pub struct Shell {
    pub bin: PathBuf,
    pub args: Vec<String>,
    pub pass_args_stdin: bool,
}

impl Shell {
    pub fn new(shell: &str) -> Self {
        match shell {
            "pwsh" | "powershell" => {
                Self {
                    bin: find_command_on_path("pwsh".into())
                        .or_else(|| find_command_on_path("powershell".into()))
                        .unwrap_or_else(|| "powershell.exe".into()),
                    args: vec![
                        "-NoLogo".into(),
                        "-Command".into(),
                        // We'll pass the command args via stdin, so that paths with special
                        // characters and spaces resolve correctly.
                        // https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pwsh?view=powershell-7.2#-command---c
                        "-".into(),
                    ],
                    pass_args_stdin: true,
                }
            }
            "bash" | "elvish" | "fish" | "sh" | "zsh" => Self {
                bin: find_command_on_path(shell.into()).unwrap_or_else(|| shell.into()),
                args: vec!["-c".into()],
                pass_args_stdin: false,
            },
            _ => unimplemented!(),
        }
    }
}

impl Default for Shell {
    fn default() -> Self {
        if consts::OS == "windows" {
            Self::new("pwsh")
        } else if let Ok(shell_bin) = env::var("SHELL") {
            Self {
                bin: shell_bin.into(),
                args: vec!["-c".into()],
                pass_args_stdin: false,
            }
        } else {
            Self::new("sh")
        }
    }
}
