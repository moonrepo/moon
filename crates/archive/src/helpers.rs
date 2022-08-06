use moon_error::{map_io_to_fs_error, MoonError};
use moon_utils::path;
use std::fs;
use std::path::{Path, PathBuf};

pub fn ensure_dir(dir: &Path) -> Result<(), MoonError> {
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| map_io_to_fs_error(e, dir.to_path_buf()))?;
    }

    Ok(())
}

pub fn prepend_name(name: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return name.to_owned();
    }

    // Use native path utils to join the paths, so we can ensure
    // the parts are joined correctly within the archive!
    let parts: PathBuf = [prefix, name].iter().collect();

    path::normalize(parts).to_string_lossy().to_string()
}
