use clean_path::Clean;
pub use dirs::home_dir as get_home_dir;
use moon_error::MoonError;
use std::path::{Path, PathBuf};

pub use pathdiff::diff_paths as relative_from;

/// If a file starts with "/", expand from the workspace root, otherwise the project root.
pub fn expand_root_path<F, P>(file: F, workspace_root: P, project_root: P) -> PathBuf
where
    F: AsRef<str>,
    P: AsRef<Path>,
{
    let file = file.as_ref();

    if file.starts_with('/') {
        workspace_root
            .as_ref()
            .join(file.strip_prefix('/').unwrap())
    } else {
        project_root.as_ref().join(file)
    }
}

pub fn normalize<T: AsRef<Path>>(path: T) -> PathBuf {
    path.as_ref().clean()
}

#[cfg(not(windows))]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

#[cfg(windows)]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('/', "\\")
}

pub fn replace_home_dir<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();

    if let Some(home_dir) = get_home_dir() {
        let home_dir_str = home_dir.to_str().unwrap_or_default();

        // Replace both forward and backward slashes
        return value
            .replace(home_dir_str, "~")
            .replace(&standardize_separators(home_dir_str), "~");
    }

    value.to_owned()
}

pub fn standardize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

pub fn to_string<T: AsRef<Path>>(path: T) -> Result<String, MoonError> {
    let mut path = path.as_ref();

    // Avoid UNC paths as they cause lots of issues
    if cfg!(windows) {
        path = dunce::simplified(&path);
    }

    match path.to_str() {
        Some(p) => Ok(p.to_owned()),
        None => Err(MoonError::PathInvalidUTF8(path.to_path_buf())),
    }
}

pub fn to_virtual_string<T: AsRef<Path>>(path: T) -> Result<String, MoonError> {
    Ok(standardize_separators(to_string(path)?))
}
