use crate::errors::ProbeError;
use flate2::read::GzDecoder;
use log::trace;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use zip::ZipArchive;

#[async_trait::async_trait]
pub trait Installable<'tool>: Send + Sync {
    /// Returns an absolute file path to the directory containing the installed tool.
    /// This is typically ~/.probe/tools/<tool>/<version>.
    fn get_install_dir(&self) -> Result<PathBuf, ProbeError>;

    /// Run any installation steps after downloading and verifying the tool.
    /// This is typically unzipping an archive, and running any installers/binaries.
    async fn install(&self, install_dir: &Path, download_path: &Path) -> Result<(), ProbeError>;
}

pub fn untar<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<&str>,
) -> Result<(), ProbeError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();
    let handle_input_error = |e: io::Error| ProbeError::Fs(input_file.to_path_buf(), e.to_string());
    let handle_output_error =
        |e: io::Error| ProbeError::Fs(output_dir.to_path_buf(), e.to_string());

    trace!(
        target: "probe:installer",
        "Unpacking tar archive {} to {}",
        input_file.to_string_lossy(),
        output_dir.to_string_lossy(),
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(handle_output_error)?;
    }

    // Open .tar.gz file
    let tar_gz = File::open(input_file).map_err(handle_input_error)?;

    // Decompress to .tar
    let tar = GzDecoder::new(tar_gz);

    // Unpack the archive into the output dir
    let mut archive = Archive::new(tar);

    for entry_result in archive.entries().map_err(handle_input_error)? {
        let mut entry = entry_result.map_err(handle_input_error)?;
        let mut path: PathBuf = entry.path().map_err(handle_input_error)?.into_owned();

        // Remove the prefix
        if let Some(prefix) = remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(path);

        // Create parent dirs
        if let Some(parent_dir) = output_path.parent() {
            fs::create_dir_all(parent_dir)
                .map_err(|e| ProbeError::Fs(parent_dir.to_path_buf(), e.to_string()))?;
        }

        entry
            .unpack(&output_path)
            .map_err(|e| ProbeError::Fs(output_path.to_path_buf(), e.to_string()))?;
    }

    Ok(())
}

pub fn unzip<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<&str>,
) -> Result<(), ProbeError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();
    let handle_input_error = |e: io::Error| ProbeError::Fs(input_file.to_path_buf(), e.to_string());
    let handle_output_error =
        |e: io::Error| ProbeError::Fs(output_dir.to_path_buf(), e.to_string());

    trace!(
        target: "probe:installer",
        "Unzipping zip archive {} to {}",
        input_file.to_string_lossy(),
        output_dir.to_string_lossy(),
    );

    if !output_dir.exists() {
        fs::create_dir_all(output_dir).map_err(handle_output_error)?;
    }

    // Open .zip file
    let zip = File::open(input_file).map_err(handle_input_error)?;

    // Unpack the archive into the output dir
    let mut archive = ZipArchive::new(zip)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let mut path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Remove the prefix
        if let Some(prefix) = remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(&path);
        let handle_error = |e: io::Error| ProbeError::Fs(output_path.to_path_buf(), e.to_string());

        // Create parent dirs
        if let Some(parent_dir) = &output_path.parent() {
            fs::create_dir_all(parent_dir)
                .map_err(|e| ProbeError::Fs(parent_dir.to_path_buf(), e.to_string()))?;
        }

        // If a folder, create the dir
        if file.is_dir() {
            fs::create_dir_all(&output_path).map_err(handle_error)?;
        }

        // If a file, copy it to the output dir
        if file.is_file() {
            let mut out = File::create(&output_path).map_err(handle_error)?;

            io::copy(&mut file, &mut out).map_err(handle_error)?;

            // Update permissions when on a nix machine
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&output_path, fs::Permissions::from_mode(mode))
                        .map_err(handle_error)?;
                }
            }
        }
    }

    Ok(())
}
