use crate::errors::ToolchainError;
use flate2::read::GzDecoder;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, trace};
use moon_utils::fs;
use moon_utils::process::{create_command, exec_command_capture_stdout};
use sha2::{Digest, Sha256};
use std::env;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use zip::ZipArchive;

pub async fn get_bin_version(bin: &Path) -> Result<String, ToolchainError> {
    let mut version = exec_command_capture_stdout(create_command(bin).args(["--version"]).env(
        "PATH",
        get_path_env_var(bin.parent().unwrap().to_path_buf()),
    ))
    .await?;

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

/// We need to ensure that our toolchain binaries are executed instead of
/// other binaries of the same name. Otherwise, tooling like nvm will
/// intercept execution and break our processes. We can work around this
/// by prepending the `PATH` environment variable.
pub fn get_path_env_var(bin_dir: PathBuf) -> std::ffi::OsString {
    let path = env::var("PATH").unwrap_or_default();
    let mut paths = vec![bin_dir];

    paths.extend(env::split_paths(&path).collect::<Vec<_>>());

    env::join_paths(paths).unwrap()
}

pub async fn download_file_from_url(url: &str, dest: &Path) -> Result<(), ToolchainError> {
    let handle_error = |e: io::Error| map_io_to_fs_error(e, dest.to_path_buf());

    trace!(
        target: "moon:toolchain",
        "Downloading file to {}",
        color::file_path(dest.parent().unwrap()),
    );

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

pub fn unpack_tar(
    input_file: &Path,
    output_dir: &Path,
    prefix: &str,
) -> Result<(), ToolchainError> {
    // Open .tar.gz file
    let tar_gz =
        File::open(input_file).map_err(|e| map_io_to_fs_error(e, input_file.to_path_buf()))?;

    // Decompress to .tar
    let tar = GzDecoder::new(tar_gz);

    // Unpack the archive into the install dir
    let mut archive = Archive::new(tar);

    archive.entries().unwrap().for_each(|entry_result| {
        let mut entry = entry_result.unwrap();

        // Remove the download folder prefix from all files
        let path = entry
            .path()
            .unwrap()
            .strip_prefix(&prefix)
            .unwrap()
            .to_owned();

        entry.unpack(&output_dir.join(path)).unwrap();
    });

    Ok(())
}

pub fn unpack_zip(
    input_file: &Path,
    output_dir: &Path,
    prefix: &str,
) -> Result<(), ToolchainError> {
    // Open .zip file
    let zip =
        File::open(input_file).map_err(|e| map_io_to_fs_error(e, input_file.to_path_buf()))?;

    // Unpack the archive into the install dir
    let mut archive = ZipArchive::new(zip).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();

        // Remove the download folder prefix from all files
        let path = match file.enclosed_name() {
            Some(path) => path.strip_prefix(&prefix).unwrap().to_owned(),
            None => continue,
        };

        let output_path = output_dir.join(&path);
        let handle_error = |e: io::Error| map_io_to_fs_error(e, output_path.to_path_buf());

        // Determine if theres a trailing slash. This is kind of nasty,
        // but we cant use `Path.ends_with()` because it only works on path parts,
        // and not the characters themself.
        let last_char = output_path.to_string_lossy();
        let has_trailing_slash = last_char.ends_with('/') || last_char.ends_with('\\');

        println!("{:#?} -> {:#?} ({})", path, output_path, has_trailing_slash);

        // If a folder, ensure it exists and continue
        if has_trailing_slash || output_path.ends_with("node_modules") {
            // `zip` is not `Send`able, so we cant use our async variant here
            std::fs::create_dir(&output_path).map_err(handle_error)?;

            // If a file, copy it to the output dir
        } else {
            let mut out = File::create(&output_path).map_err(handle_error)?;

            io::copy(&mut file, &mut out).map_err(handle_error)?;
        }

        // Update permissions when on a nix machine
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&output_path, std::fs::Permissions::from_mode(mode))
                    .map_err(handle_error)?;
            }
        }
    }

    Ok(())
}

pub async fn unpack(
    input_file: &Path,
    output_dir: &Path,
    prefix: &str,
) -> Result<(), ToolchainError> {
    fs::create_dir_all(output_dir).await?;

    println!("Input = {:#?}", input_file,);
    println!("Output = {:#?}", output_dir);
    println!("Prefix = {:#?}", prefix);

    if input_file.extension().unwrap() == "zip" {
        unpack_zip(input_file, output_dir, prefix)?;
    } else {
        unpack_tar(input_file, output_dir, prefix)?;
    }

    Ok(())
}
