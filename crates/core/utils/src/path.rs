use clean_path::Clean;
use miette::IntoDiagnostic;
use relative_path::PathExt;
use std::path::{Path, PathBuf};

pub use pathdiff::diff_paths as relative_from;

/// If a file starts with "/", expand from the workspace root, otherwise the project root.
#[inline]
pub fn expand_to_workspace_relative<F, P>(file: F, workspace_root: P, project_root: P) -> PathBuf
where
    F: AsRef<str>,
    P: AsRef<Path>,
{
    let file = file.as_ref();
    let workspace_root = workspace_root.as_ref();
    let project_root = project_root.as_ref();

    if let Some(ws_rel_file) = file.strip_prefix('/') {
        return PathBuf::from(normalize_separators(ws_rel_file));
    }

    let project_source = project_root.strip_prefix(workspace_root).unwrap();

    if let Some(negative_glob) = file.strip_prefix('!') {
        return PathBuf::from(format!("!{}", project_source.to_string_lossy()))
            .join(normalize_separators(negative_glob));
    }

    project_source.join(normalize_separators(file))
}

#[inline]
pub fn normalize<T: AsRef<Path>>(path: T) -> PathBuf {
    path.as_ref().clean()
}

#[cfg(not(windows))]
#[inline]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

#[cfg(windows)]
#[inline]
pub fn normalize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('/', "\\")
}

#[inline]
pub fn replace_home_dir<T: AsRef<str>>(value: T) -> String {
    let value = value.as_ref();

    if let Some(home_dir) = starbase_utils::dirs::home_dir() {
        let home_dir_str = home_dir.to_str().unwrap_or_default();

        // Replace both forward and backward slashes
        return value
            .replace(home_dir_str, "~")
            .replace(&standardize_separators(home_dir_str), "~");
    }

    value.to_owned()
}

#[inline]
pub fn standardize_separators<T: AsRef<str>>(path: T) -> String {
    path.as_ref().replace('\\', "/")
}

#[inline]
pub fn to_string<T: AsRef<Path>>(path: T) -> miette::Result<String> {
    let mut path = path.as_ref();

    // Avoid UNC paths as they cause lots of issues
    if cfg!(windows) {
        path = dunce::simplified(path);
    }

    match path.to_str() {
        Some(p) => Ok(p.to_owned()),
        None => Err(miette::miette!(
            "Path {} contains invalid UTF-8 characters.",
            path.display()
        )),
    }
}

#[inline]
pub fn to_virtual_string<T: AsRef<Path>>(path: T) -> miette::Result<String> {
    Ok(standardize_separators(to_string(path)?))
}

#[inline]
pub fn to_relative_virtual_string<F: AsRef<Path>, T: AsRef<Path>>(
    from: F,
    to: T,
) -> miette::Result<String> {
    let value = from
        .as_ref()
        .relative_to(to.as_ref())
        .into_diagnostic()?
        .to_string();

    Ok(if value.is_empty() { ".".into() } else { value })
}
