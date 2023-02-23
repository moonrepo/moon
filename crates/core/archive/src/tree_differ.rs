use moon_error::MoonError;
use moon_utils::{fs, glob};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::{
    fs::File,
    io::{BufReader, Read},
};

pub struct TreeDiffer {
    /// A mapping of all files in the destination directory
    /// to their current file sizes.
    pub files: FxHashMap<PathBuf, u64>,
}

impl TreeDiffer {
    /// Load the tree at the defined destination root and scan the file system
    /// using the defined lists of paths, either files or folders. If a folder,
    /// recursively scan all files and create an internal manifest to track diffing.
    pub fn load(dest_root: &Path, paths: &[String]) -> Result<Self, MoonError> {
        let mut files = FxHashMap::default();

        let mut track = |file: PathBuf| {
            if file.exists() {
                let size = match std::fs::metadata(&file) {
                    Ok(meta) => meta.len(),
                    Err(_) => 0,
                };

                files.insert(file, size);
            }
        };

        for path in paths {
            if glob::is_glob(path) {
                for file in glob::walk_files(dest_root, [path])
                    .map_err(|e| MoonError::Generic(e.to_string()))?
                {
                    track(file);
                }
            } else {
                let path = dest_root.join(path);

                if path.is_file() {
                    track(path);
                } else if path.is_dir() {
                    for file in fs::read_dir_all(path)? {
                        track(file.path());
                    }
                }
            }
        }

        Ok(TreeDiffer { files })
    }

    /// Compare 2 files byte by byte and return true if both files are equal.
    pub fn are_files_equal<S: Read, D: Read>(
        &self,
        source: &mut S,
        dest: &mut D,
    ) -> Result<bool, MoonError> {
        let mut areader = BufReader::new(source);
        let mut breader = BufReader::new(dest);
        let mut abuf = [0; 512];
        let mut bbuf = [0; 512];

        while let (Ok(av), Ok(bv)) = (areader.read(&mut abuf), breader.read(&mut bbuf)) {
            // We've reached the end of the file for either one
            if av < 512 || bv < 512 {
                return Ok(abuf == bbuf);
            }

            // Otherwise, compare buffer
            if abuf != bbuf {
                return Ok(false);
            }
        }

        Ok(false)
    }

    /// Remove all files in the destination directory that have not been
    /// overwritten with a source file, or are the same size as a source file.
    /// We can assume these are stale artifacts that should no longer exist!
    pub fn remove_stale_tracked_files(&mut self) {
        for (file, _) in self.files.drain() {
            let _ = std::fs::remove_file(file);
        }
    }

    /// Determine whether the source should be written to the destination.
    /// If a file exists at the destination, run a handful of checks to
    /// determine whether we overwrite the file or keep it (equal content).
    pub fn should_write_source<T: Read>(
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
        let mut dest = File::open(dest_path)?;

        Ok(!self.are_files_equal(source, &mut dest)?)
    }

    /// Untrack a destination file from the internal registry.
    pub fn untrack_file(&mut self, dest: &Path) {
        self.files.remove(dest);
    }
}
