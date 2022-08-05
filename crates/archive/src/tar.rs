use crate::helpers::{ensure_dir, prepend_name};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use moon_error::{map_io_to_fs_error, MoonError};
use moon_logger::{color, debug, trace};
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

const TARGET: &str = "moon:archive:tar";

#[track_caller]
pub fn tar<I: AsRef<Path>, O: AsRef<Path>>(
    input_src: I,
    output_file: O,
    base_prefix: Option<&str>,
) -> Result<(), MoonError> {
    let input_src = input_src.as_ref();
    let output_file = output_file.as_ref();

    debug!(
        target: TARGET,
        "Packing tar archive with {} to {}",
        color::path(input_src),
        color::path(output_file),
    );

    // Create .tar
    let tar =
        File::create(output_file).map_err(|e| map_io_to_fs_error(e, output_file.to_path_buf()))?;

    // Compress to .tar.gz
    let tar_gz = GzEncoder::new(tar, Compression::fast());

    // Add the files to the archive
    let mut archive = Builder::new(tar_gz);

    if input_src.is_file() {
        let mut file =
            File::open(input_src).map_err(|e| map_io_to_fs_error(e, input_src.to_path_buf()))?;
        let file_name = input_src
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();
        let prefix = base_prefix.unwrap_or_default();

        archive.append_file(prepend_name(file_name, prefix), &mut file)?;

        trace!(target: TARGET, "Packing file {}", color::path(&input_src));
    } else {
        archive.append_dir_all(base_prefix.unwrap_or("."), input_src)?;
    }

    archive.finish()?;

    Ok(())
}

#[track_caller]
pub fn untar<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<&str>,
) -> Result<(), MoonError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    debug!(
        target: TARGET,
        "Unpacking tar archive {} to {}",
        color::path(input_file),
        color::path(output_dir),
    );

    ensure_dir(output_dir)?;

    // Open .tar.gz file
    let tar_gz =
        File::open(input_file).map_err(|e| map_io_to_fs_error(e, input_file.to_path_buf()))?;

    // Decompress to .tar
    let tar = GzDecoder::new(tar_gz);

    // Unpack the archive into the output dir
    let mut archive = Archive::new(tar);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let mut path: PathBuf = entry.path()?.to_owned().to_path_buf();

        // Remove the prefix
        if let Some(prefix) = remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(&prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(path);

        // Create parent dirs
        if let Some(parent_dir) = output_path.parent() {
            ensure_dir(parent_dir)?;
        }

        entry.unpack(&output_path)?;

        trace!(
            target: TARGET,
            "Unpacking file {}",
            color::path(&output_path)
        );
    }

    Ok(())
}
