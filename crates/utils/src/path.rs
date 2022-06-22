pub use dirs::home_dir as get_home_dir;
use moon_error::MoonError;
use path_clean::PathClean;
use std::path::{Path, PathBuf};

/// If a file starts with "/", expand from the workspace root, otherwise the project root.
pub fn expand_root_path(file: &str, workspace_root: &Path, project_root: &Path) -> PathBuf {
    if file.starts_with('/') {
        workspace_root.join(file.strip_prefix('/').unwrap())
    } else {
        project_root.join(file)
    }
}

pub fn normalize(path: &Path) -> PathBuf {
    path.to_path_buf().clean()
}

#[cfg(not(windows))]
pub fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(windows)]
pub fn normalize_separators(path: &str) -> String {
    path.replace('/', "\\")
}

pub fn path_to_string(path: &Path) -> Result<String, MoonError> {
    match path.to_str() {
        Some(p) => Ok(p.to_owned()),
        None => Err(MoonError::PathInvalidUTF8(path.to_path_buf())),
    }
}

pub fn replace_home_dir(value: &str) -> String {
    if let Some(home_dir) = get_home_dir() {
        let home_dir_str = home_dir.to_str().unwrap_or_default();

        // Replace both forward and backward slashes
        return value
            .replace(home_dir_str, "~")
            .replace(&standardize_separators(home_dir_str), "~");
    }

    value.to_owned()
}

pub fn standardize_separators(path: &str) -> String {
    path.replace('\\', "/")
}
