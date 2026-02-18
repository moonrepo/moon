use proto_shim::exec_command_and_replace;
use std::env;
use std::ffi::OsString;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    sigpipe::reset();

    let mut args = env::args_os().collect::<Vec<_>>();
    args[0] = OsString::from("exec");

    let mut command = Command::new("moon");
    command.args(args);

    exec_command_and_replace(command)
}
