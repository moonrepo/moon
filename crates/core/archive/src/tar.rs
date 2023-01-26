use crate::errors::ArchiveError;
use crate::helpers::{ensure_dir, prepend_name};
use crate::tree_differ::TreeDiffer;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use moon_error::map_io_to_fs_error;
use moon_logger::{color, debug, trace};
use moon_utils::fs;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

const LOG_TARGET: &str = "moon:archive:tar";

pub struct TarArchiver<'l> {
    input_root: &'l Path,

    output_file: &'l Path,

    prefix: &'l str,

    // relative file in tarball -> absolute file path to source
    sources: FxHashMap<String, PathBuf>,
}

impl<'l> TarArchiver<'l> {
    pub fn new(input_root: &'l Path, output_file: &'l Path) -> Self {
        TarArchiver {
            input_root,
            output_file,
            prefix: "",
            sources: FxHashMap::default(),
        }
    }

    pub fn add_source<P: AsRef<Path>>(&mut self, source: P, name: Option<&str>) -> &mut Self {
        let source = source.as_ref();
        let name = match name {
            Some(n) => n.to_owned(),
            None => fs::file_name(source),
        };

        self.sources.insert(name, source.to_path_buf());
        self
    }

    pub fn set_prefix(&mut self, prefix: &'l str) -> &mut Self {
        self.prefix = prefix;
        self
    }

    pub fn pack(&self) -> Result<(), ArchiveError> {
        debug!(
            target: LOG_TARGET,
            "Packing tar archive from {} to {}",
            color::path(self.input_root),
            color::path(self.output_file),
        );

        // Create .tar
        let tar = File::create(self.output_file)
            .map_err(|e| map_io_to_fs_error(e, self.output_file.to_owned()))?;

        // Compress to .tar.gz
        let tar_gz = GzEncoder::new(tar, Compression::fast());

        // Add the files to the archive
        let mut archive = Builder::new(tar_gz);

        for (file, source) in &self.sources {
            if !source.exists() {
                trace!(
                    target: LOG_TARGET,
                    "Source file {} does not exist, skipping",
                    color::path(source)
                );

                continue;
            }

            if source.is_file() {
                trace!(target: LOG_TARGET, "Packing file {}", color::path(source));

                let mut fh =
                    File::open(source).map_err(|e| map_io_to_fs_error(e, source.to_path_buf()))?;

                archive.append_file(prepend_name(file, self.prefix), &mut fh)?;
            } else {
                trace!(
                    target: LOG_TARGET,
                    "Packing directory {}",
                    color::path(source)
                );

                archive.append_dir_all(prepend_name(file, self.prefix), source)?;
            }
        }

        archive.finish()?;

        Ok(())
    }
}

#[track_caller]
pub fn tar<I: AsRef<Path>, O: AsRef<Path>>(
    input_root: I,
    files: &[String],
    output_file: O,
    base_prefix: Option<&str>,
) -> Result<(), ArchiveError> {
    let input_root = input_root.as_ref();
    let mut tar = TarArchiver::new(input_root, output_file.as_ref());

    if let Some(prefix) = base_prefix {
        tar.set_prefix(prefix);
    }

    for file in files {
        tar.add_source(input_root.join(file), Some(file));
    }

    tar.pack()?;

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
        let mut path: PathBuf = entry.path()?.into_owned();

        // Remove the prefix
        if let Some(prefix) = remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(path);

        // Create parent dirs
        if let Some(parent_dir) = output_path.parent() {
            ensure_dir(parent_dir)?;
        }

        entry.unpack(&output_path)?;
    }

    Ok(())
}

#[track_caller]
pub fn untar_with_diff<I: AsRef<Path>, O: AsRef<Path>>(
    differ: &mut TreeDiffer,
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
        let mut path: PathBuf = entry.path()?.into_owned();

        // Remove the prefix
        if let Some(prefix) = remove_prefix {
            if path.starts_with(prefix) {
                path = path.strip_prefix(prefix).unwrap().to_owned();
            }
        }

        let output_path = output_dir.join(path);

        // Create parent dirs
        if let Some(parent_dir) = output_path.parent() {
            ensure_dir(parent_dir)?;
        }

        // Unpack the file if different than destination
        if differ.should_write_source(entry.size(), &mut entry, &output_path)? {
            entry.unpack(&output_path)?;
        }

        differ.untrack_file(&output_path);
    }

    differ.remove_stale_tracked_files();

    Ok(())
}
