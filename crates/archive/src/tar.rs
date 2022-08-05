use crate::errors::ArchiveError;
use crate::helpers::{ensure_dir, prepend_name};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, map_list, trace};
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

const LOG_TARGET: &str = "moon:archive:tar";

#[track_caller]
pub fn tar<I: AsRef<Path>, O: AsRef<Path>>(
    input_root: I,
    files: &[String],
    output_file: O,
    base_prefix: Option<&str>,
) -> Result<(), ArchiveError> {
    let input_root = input_root.as_ref();
    let output_file = output_file.as_ref();

    debug!(
        target: LOG_TARGET,
        "Packing tar archive from {} with {} to {}",
        color::path(input_root),
        map_list(files, |f| color::file(f)),
        color::path(output_file),
    );

    // Create .tar
    let tar =
        File::create(output_file).map_err(|e| map_io_to_fs_error(e, output_file.to_path_buf()))?;

    // Compress to .tar.gz
    let tar_gz = GzEncoder::new(tar, Compression::fast());

    // Add the files to the archive
    let mut archive = Builder::new(tar_gz);
    let prefix = base_prefix.unwrap_or_default();

    for file in files {
        let input_src = input_root.join(file);
        let name = input_src
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        if input_src.is_file() {
            trace!(
                target: LOG_TARGET,
                "Packing file {}",
                color::path(&input_src)
            );

            let mut file = File::open(&input_src)
                .map_err(|e| map_io_to_fs_error(e, input_src.to_path_buf()))?;

            archive.append_file(prepend_name(name, prefix), &mut file)?;
        } else {
            trace!(
                target: LOG_TARGET,
                "Packing directory {}",
                color::path(&input_src)
            );

            archive.append_dir_all(prepend_name(name, prefix), input_src)?;
        }
    }

    archive.finish()?;

    Ok(())
}

#[track_caller]
pub fn untar<I: AsRef<Path>, O: AsRef<Path>>(
    input_file: I,
    output_dir: O,
    remove_prefix: Option<&str>,
) -> Result<(), ArchiveError> {
    let input_file = input_file.as_ref();
    let output_dir = output_dir.as_ref();

    debug!(
        target: LOG_TARGET,
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
            target: LOG_TARGET,
            "Unpacking file {}",
            color::path(&output_path)
        );
    }

    Ok(())
}
