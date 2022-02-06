use crate::fs::get_home_dir;
use moon_error::{map_io_to_process_error, MoonError};
use moon_logger::{color, trace};
use std::ffi::OsStr;
use tokio::process::Command;

pub use std::process::Output;

pub fn create_command<S: AsRef<OsStr>>(bin: S) -> Command {
    Command::new(bin)
}

pub async fn exec_command(command: &mut Command) -> Result<Output, MoonError> {
    let output = command.output(); // Start immediately

    let base_command = command.as_std();
    let bin_name = base_command.get_program().to_str().unwrap_or("<unknown>");
    let args_list = base_command
        .get_args()
        .into_iter()
        .map(|a| a.to_str().unwrap())
        .collect::<Vec<_>>();
    let command_line = format!("{} {}", bin_name, args_list.join(" "))
        .replace(get_home_dir().unwrap_or_default().to_str().unwrap(), "~");

    if let Some(cwd) = base_command.get_current_dir() {
        trace!(
            target: "moon:utils",
            "Running command {} (in {})",
            color::shell(&command_line),
            color::file_path(cwd),
        );
    } else {
        trace!(
            target: "moon:utils",
            "Running command {} ",
            color::shell(&command_line),
        );
    }

    Ok(output
        .await
        .map_err(|e| map_io_to_process_error(e, bin_name))?)
}

pub async fn exec_command_capture_stdout(command: &mut Command) -> Result<String, MoonError> {
    let output = exec_command(command).await?;

    Ok(output_to_string(&output.stdout))
}

pub fn output_to_string(data: &[u8]) -> String {
    String::from_utf8(data.to_vec()).unwrap_or_default()
}
