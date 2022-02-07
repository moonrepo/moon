use crate::errors::ToolchainError;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, trace};
use moon_utils::fs;
use moon_utils::process::{create_command, exec_command_capture_stdout};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

pub fn is_ci() -> bool {
    env::var("CI").is_ok()
}

pub async fn get_bin_version(bin: &Path) -> Result<String, ToolchainError> {
    let mut version = exec_command_capture_stdout(create_command(bin).args(["--version"])).await?;

    version = version.trim().to_owned();

    if version.is_empty() {
        version = String::from("0.0.0");
    }

    if version.starts_with('v') {
        version = version.replace('v', "");
    }

    Ok(version)
}

pub fn get_file_sha256_hash(path: &Path) -> Result<String, ToolchainError> {
    let handle_error = |e: io::Error| map_io_to_fs_error(e, path.to_path_buf());

    let mut file = File::open(path).map_err(handle_error)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(handle_error)?;

    let hash = format!("{:x}", sha.finalize());

    trace!(
        target: "moon:toolchain",
        "Calculating sha256 for file {} -> {}",
        color::file_path(path),
        color::symbol(&hash)
    );

    Ok(hash)
}

pub async fn download_file_from_url(url: &str, dest: &Path) -> Result<(), ToolchainError> {
    let handle_error = |e: io::Error| map_io_to_fs_error(e, dest.to_path_buf());

    // Ensure parent directories exist
    fs::create_dir_all(dest.parent().unwrap()).await?;

    // Fetch the file from the HTTP source
    let response = reqwest::get(url).await?;

    // Write the bytes to our local file
    let mut contents = io::Cursor::new(response.bytes().await?);
    let mut file = File::create(dest).map_err(handle_error)?;

    io::copy(&mut contents, &mut file).map_err(handle_error)?;

    Ok(())
}
