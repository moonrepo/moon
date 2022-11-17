use moon_error::MoonError;
use moon_utils::fs;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::{
    fs::File,
    io::{BufReader, Read},
};

pub struct TreeDiffer {
    /// A mapping of all files in the destination directory
    /// to their current file sizes.
    files: FxHashMap<PathBuf, u64>,
}

impl TreeDiffer {
    pub async fn load(dest_root: &Path, paths: &[String]) -> Result<Self, MoonError> {
        let mut files = vec![];

        for path in paths {
            let path = dest_root.join(path);

            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                for file in fs::read_dir_all(path).await? {
                    files.push(file.path());
                }
            }
        }

        let mut tracked = FxHashMap::default();

        for file in files {
            if !file.exists() {
                continue;
            }

            let size = match fs::metadata(&file).await {
                Ok(meta) => meta.len(),
                Err(_) => 0,
            };

            tracked.insert(file, size);
        }

        Ok(TreeDiffer { files: tracked })
    }

    /// Remove all files in the destination directory that have not been
    /// overwritten with a source file, or are the same size as a source file.
    /// We can assume these are stale artifacts that should no longer exist!
    pub async fn remove_stale_files(&mut self) -> Result<(), MoonError> {
        for (file, _) in self.files.drain() {
            fs::remove(file).await?;
        }

        Ok(())
    }

    /// Determine whether the source should be written to the destination.
    /// If a file exists at the destination, run a handful of checks to
    /// determine whether we overwrite the file or keep it (equal content).
    pub fn should_write<T: Read>(
        &self,
        source_size: u64,
        source: &mut T,
        dest_path: &Path,
    ) -> Result<bool, MoonError> {
        // If the destination doesn't exist, always use the source
        if !dest_path.exists() || !self.files.contains_key(dest_path) {
            return Ok(true);
        }

        // If the file sizes are different, use the source
        let Some(dest_size) = self.files.get(dest_path) else {
            return Ok(true);
        };

        if source_size != *dest_size {
            return Ok(true);
        }

        // If the file sizes are the same, compare byte ranges to determine a difference
        Ok(!self.are_files_equal(source, dest_path)?)
    }

    /// Untrack a destination file from the internal registry.
    pub fn untrack(&mut self, dest: &Path) {
        self.files.remove(dest);
    }

    /// Compare 2 files byte by byte and return true if both files are equal.
    fn are_files_equal<T: Read>(
        &self,
        source: &mut T,
        dest_path: &Path,
    ) -> Result<bool, MoonError> {
        let mut areader = BufReader::new(source);
        let mut breader = BufReader::new(File::open(dest_path)?);
        let mut abuf = [0; 512];
        let mut bbuf = [0; 512];

        loop {
            match (areader.read(&mut abuf), breader.read(&mut bbuf)) {
                (Ok(av), Ok(bv)) => {
                    // We've reached the end of the file for either one
                    if av < 512 || bv < 512 {
                        return Ok(abuf == bbuf);
                    }

                    // Otherwise, compare buffer
                    if abuf != bbuf {
                        return Ok(false);
                    }
                }
                _ => {
                    break;
                }
            }
        }

        Ok(false)
    }
}
