use proto_shim::exec_command_and_replace;
use std::collections::VecDeque;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    sigpipe::reset();

    // Collect existing `PATH`
    let path_env = env::var_os("PATH").unwrap_or_default();
    let mut paths = env::split_paths(&path_env).collect::<VecDeque<_>>();

    // Collect arguments
    let mut args = env::args_os().collect::<Vec<_>>();

    // Extract the directory of the current executable
    let current_exe = PathBuf::from(&args[0]);

    if let Some(exe_dir) = current_exe.parent() {
        paths.push_front(exe_dir.into());
    }

    // Replace the current executable with `exec` subcommand
    args[0] = OsString::from("exec");

    // Create the new `moon` command
    let mut command = Command::new("moon");
    command.args(args);

    if let Ok(path_env) = env::join_paths(paths) {
        command.env("PATH", path_env);
    }

    exec_command_and_replace(command)
}
