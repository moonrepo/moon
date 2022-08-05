use moon_error::{map_io_to_fs_error, MoonError};
use std::fs;
use std::path::Path;

pub fn ensure_dir(dir: &Path) -> Result<(), MoonError> {
    if !dir.exists() {
        fs::create_dir_all(dir).map_err(|e| map_io_to_fs_error(e, dir.to_path_buf()))?;
    }

    Ok(())
}

pub fn prepend_name(name: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{}/{}", prefix, name)
    }
}
