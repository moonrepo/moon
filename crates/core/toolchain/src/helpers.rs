use crate::errors::ToolchainError;
use moon_archive::{untar, unzip};
use moon_error::map_io_to_fs_error;
use moon_logger::{color, trace};
use moon_utils::fs;
use moon_utils::process::{output_to_trimmed_string, Command};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io;
use std::path::Path;

pub const LOG_TARGET: &str = "moon:toolchain";

pub async fn get_bin_version(bin: &Path) -> Result<String, ToolchainError> {
    let output = Command::new(bin)
        .arg("--version")
        .env("PATH", get_path_env_var(bin.parent().unwrap()))
        .exec_capture_output()
        .await?;

    let mut version = output_to_trimmed_string(&output.stdout);

    if version.is_empty() {
        version = String::from("0.0.0");
    }

    if version.starts_with('v') {
        version = version.replace('v', "");
    }

    Ok(version)
}

pub fn get_file_sha256_hash(path: &Path) -> Result<String, ToolchainError> {
    trace!(
        target: LOG_TARGET,
        "Calculating sha256 for file {}",
        color::path(path),
    );

    let handle_error = |e: io::Error| map_io_to_fs_error(e, path.to_path_buf());

    let mut file = File::open(path).map_err(handle_error)?;
    let mut sha = Sha256::new();

    io::copy(&mut file, &mut sha).map_err(handle_error)?;

    let hash = format!("{:x}", sha.finalize());

    trace!(
        target: LOG_TARGET,
        "Calculated hash {}",
        color::symbol(&hash)
    );

    Ok(hash)
}

/// We need to ensure that our toolchain binaries are executed instead of
/// other binaries of the same name. Otherwise, tooling like nvm will
/// intercept execution and break our processes. We can work around this
/// by prepending the `PATH` environment variable.
pub fn get_path_env_var(bin_dir: &Path) -> std::ffi::OsString {
    let path = env::var("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir.to_path_buf()];

    paths.extend(env::split_paths(&path).collect::<Vec<_>>());

    env::join_paths(paths).unwrap()
}

pub async fn download_file_from_url<T: AsRef<str>>(
    url: T,
    dest: &Path,
) -> Result<(), ToolchainError> {
    let url = url.as_ref();
    let handle_error = |e: io::Error| map_io_to_fs_error(e, dest.to_path_buf());

    trace!(
        target: LOG_TARGET,
        "Downloading file {} to {}",
        color::url(url),
        color::path(dest),
    );

    // Ensure parent directories exist
    fs::create_dir_all(dest.parent().unwrap()).await?;

    // Fetch the file from the HTTP source
    let response = reqwest::get(url).await?;
    let status = response.status();

    if !status.is_success() {
        return Err(ToolchainError::DownloadFailed(
            url.to_owned(),
            status.to_string(),
        ));
    }

    // Write the bytes to our local file
    let mut contents = io::Cursor::new(response.bytes().await?);
    let mut file = File::create(dest).map_err(handle_error)?;

    io::copy(&mut contents, &mut file).map_err(handle_error)?;

    Ok(())
}

pub async fn unpack(
    input_file: &Path,
    output_dir: &Path,
    prefix: &str,
) -> Result<(), ToolchainError> {
    fs::create_dir_all(output_dir).await?;

    if input_file.extension().unwrap() == "zip" {
        unzip(input_file, output_dir, Some(prefix))?;
    } else {
        untar(input_file, output_dir, Some(prefix))?;
    }

    Ok(())
}
