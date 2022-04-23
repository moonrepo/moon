use async_recursion::async_recursion;
use globset::GlobSet;
use json_comments::StripComments;
use moon_error::{map_io_to_fs_error, map_json_to_error, MoonError};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Read;
use std::path::Path;
use tokio::fs;

pub fn clean_json(json: String) -> Result<String, MoonError> {
    // Remove comments
    let mut stripped = String::with_capacity(json.len());

    StripComments::new(json.as_bytes())
        .read_to_string(&mut stripped)
        .map_err(MoonError::Unknown)?;

    // Remove trailing commas
    let stripped = Regex::new(r",(?P<valid>\s*})")
        .unwrap()
        .replace_all(&stripped, "$valid");

    Ok(String::from(stripped))
}

pub async fn create_dir_all(path: &Path) -> Result<(), MoonError> {
    if !path.exists() {
        fs::create_dir_all(&path)
            .await
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

pub async fn link_file(from_root: &Path, from: &Path, to_root: &Path) -> Result<(), MoonError> {
    let to = to_root.join(from.strip_prefix(from_root).unwrap());

    // Hardlink has already been created
    if to.exists() {
        return Ok(());
    }

    let to_dir = to.parent().unwrap();

    if to_dir != to_root {
        create_dir_all(to_dir).await?;
    }

    fs::hard_link(from, &to)
        .await
        .map_err(|_| MoonError::HardLink(from.to_path_buf(), to.clone()))?;

    Ok(())
}

#[async_recursion]
pub async fn link_dir(from_root: &Path, from: &Path, to_root: &Path) -> Result<(), MoonError> {
    let entries = read_dir(from).await?;
    let mut dirs = vec![];

    // Link files before dirs incase an error occurs
    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            link_file(from_root, &path, to_root).await?;
        } else {
            dirs.push(path);
        }
    }

    // Link dirs in sequence for the same reason
    for dir in dirs {
        link_dir(from_root, &dir, to_root).await?;
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn matches_globset(globset: &GlobSet, path: &Path) -> bool {
    globset.is_match(path)
}

// globset doesnt match against inputs that use backwards slashes
// https://github.com/BurntSushi/ripgrep/issues/2001
#[cfg(windows)]
pub fn matches_globset(globset: &GlobSet, path: &Path) -> bool {
    globset.is_match(&PathBuf::from(normalize_glob(path)))
}

pub async fn metadata(path: &Path) -> Result<std::fs::Metadata, MoonError> {
    fs::metadata(path)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))
}

pub async fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>, MoonError> {
    let handle_error = |e| map_io_to_fs_error(e, path.to_path_buf());

    let mut entries = fs::read_dir(path).await.map_err(handle_error)?;
    let mut results = vec![];

    while let Some(entry) = entries.next_entry().await.map_err(handle_error)? {
        results.push(entry);
    }

    Ok(results)
}

#[async_recursion]
pub async fn read_dir_all(path: &Path) -> Result<Vec<fs::DirEntry>, MoonError> {
    let handle_error = |e| map_io_to_fs_error(e, path.to_path_buf());

    let mut entries = fs::read_dir(path).await.map_err(handle_error)?;
    let mut results = vec![];

    while let Some(entry) = entries.next_entry().await.map_err(handle_error)? {
        if let Ok(file_type) = entry.file_type().await {
            if file_type.is_dir() {
                results.extend(read_dir_all(&entry.path()).await?);
            } else {
                results.push(entry);
            }
        }
    }

    Ok(results)
}

pub async fn read_json<T>(path: &Path) -> Result<T, MoonError>
where
    T: DeserializeOwned,
{
    let contents = read_json_string(path).await?;

    let json: T =
        serde_json::from_str(&contents).map_err(|e| map_json_to_error(e, path.to_path_buf()))?;

    Ok(json)
}

pub async fn read_json_string(path: &Path) -> Result<String, MoonError> {
    let json = fs::read_to_string(path)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    clean_json(json)
}

pub async fn remove_file(path: &Path) -> Result<(), MoonError> {
    if path.exists() {
        fs::remove_file(&path)
            .await
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

pub async fn remove_dir_all(path: &Path) -> Result<(), MoonError> {
    if path.exists() {
        fs::remove_dir_all(&path)
            .await
            .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

pub async fn write(path: &Path, data: impl AsRef<[u8]>) -> Result<(), MoonError> {
    fs::write(path, data)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}

pub async fn write_json<T>(path: &Path, json: &T, pretty: bool) -> Result<(), MoonError>
where
    T: ?Sized + Serialize,
{
    let data = if pretty {
        serde_json::to_string_pretty(&json).map_err(|e| map_json_to_error(e, path.to_path_buf()))?
    } else {
        serde_json::to_string(&json).map_err(|e| map_json_to_error(e, path.to_path_buf()))?
    };

    fs::write(path, data)
        .await
        .map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}
