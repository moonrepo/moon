pub mod fs;
pub mod regex;
pub mod test;

use moon_logger::{color, trace};
use std::io::Error;
use std::path::Path;
use std::process::Output;
use tokio::process::Command;

#[macro_export]
macro_rules! string_vec {
    () => {{
        Vec::<String>::new()
    }};
    ($($item:expr),+ $(,)?) => {{
        vec![
            $( String::from($item), )*
        ]
    }};
}

pub fn output_to_string(data: Vec<u8>) -> String {
    String::from_utf8(data).unwrap_or_default().to_owned()
}

pub async fn exec_bin_in_dir(file: &Path, args: Vec<&str>, dir: &Path) -> Result<Output, Error> {
    Ok(exec_command_in_dir(file.to_str().unwrap(), args, dir).await?)
}

pub async fn exec_bin_with_output(file: &Path, args: Vec<&str>) -> Result<String, Error> {
    Ok(exec_command_with_output(file.to_str().unwrap(), args).await?)
}

pub async fn exec_command_in_dir(bin: &str, args: Vec<&str>, dir: &Path) -> Result<Output, Error> {
    let command_line = format!("{} {}", bin, args.join(" "));

    trace!(
        target: "moon:utils",
        "Running command {} in {}",
        color::shell(&command_line),
        color::file_path(dir),
    );

    let output = Command::new(bin).args(args).current_dir(dir).output();

    Ok(output.await?)
}

pub async fn exec_command_with_output(bin: &str, args: Vec<&str>) -> Result<String, Error> {
    let command_line = format!("{} {}", bin, args.join(" "));

    trace!(
        target: "moon:utils",
        "Running command {} and returning output",
        color::shell(&command_line),
    );

    let output = Command::new(bin).args(args).output();

    Ok(output_to_string(output.await?.stdout))
}
