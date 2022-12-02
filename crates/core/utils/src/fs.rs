use moon_error::{map_io_to_fs_error, MoonError};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[inline]
pub fn copy_file<S: AsRef<Path>, D: AsRef<Path>>(from: S, to: D) -> Result<(), MoonError> {
    let from = from.as_ref();
    let to = to.as_ref();
    let to_dir = to.parent().unwrap();

    create_dir_all(to_dir)?;

    fs::copy(from, to).map_err(|e| map_io_to_fs_error(e, from.to_path_buf()))?;

    Ok(())
}

pub fn copy_dir_all<T: AsRef<Path> + Send>(
    from_root: T,
    from: T,
    to_root: T,
) -> Result<(), MoonError> {
    let from_root = from_root.as_ref();
    let from = from.as_ref();
    let to_root = to_root.as_ref();
    let entries = read_dir(from)?;
    let mut dirs = vec![];

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            copy_file(
                path.to_owned(),
                to_root.join(path.strip_prefix(from_root).unwrap()),
            )?;
        } else if path.is_dir() {
            dirs.push(path);
        }
    }

    for dir in dirs {
        copy_dir_all(from_root, &dir, to_root)?;
    }

    Ok(())
}

#[inline]
pub fn create_dir_all<T: AsRef<Path>>(path: T) -> Result<(), MoonError> {
    let path = path.as_ref();

    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

#[inline]
pub fn find_upwards<F, P>(name: F, dir: P) -> Option<PathBuf>
where
    F: AsRef<str>,
    P: AsRef<Path>,
{
    let dir = dir.as_ref();
    let findable = dir.join(name.as_ref());

    if findable.exists() {
        return Some(findable);
    }

    match dir.parent() {
        Some(parent_dir) => find_upwards(name, parent_dir),
        None => None,
    }
}

pub struct EditorConfigProps {
    pub eof: String,
    pub indent: String,
}

pub fn get_editor_config_props<T: AsRef<Path>>(path: T) -> EditorConfigProps {
    use ec4rs::property::*;

    let editor_config = ec4rs::properties_of(path).unwrap_or_default();
    let tab_width = editor_config
        .get::<TabWidth>()
        .unwrap_or(TabWidth::Value(4));
    let indent_size = editor_config
        .get::<IndentSize>()
        .unwrap_or(IndentSize::Value(2));
    let indent_style = editor_config
        .get::<IndentStyle>()
        .unwrap_or(IndentStyle::Spaces);
    let insert_final_newline = editor_config
        .get::<FinalNewline>()
        .unwrap_or(FinalNewline::Value(true));

    EditorConfigProps {
        eof: if matches!(insert_final_newline, FinalNewline::Value(true)) {
            "\n".into()
        } else {
            "".into()
        },
        indent: match indent_style {
            IndentStyle::Tabs => "\t".into(),
            IndentStyle::Spaces => match indent_size {
                IndentSize::UseTabWidth => match tab_width {
                    TabWidth::Value(value) => " ".repeat(value),
                },
                IndentSize::Value(value) => " ".repeat(value),
            },
        },
    }
}

#[inline]
pub fn metadata<T: AsRef<Path>>(path: T) -> Result<fs::Metadata, MoonError> {
    let path = path.as_ref();

    fs::metadata(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))
}

#[inline]
pub fn read_dir<T: AsRef<Path>>(path: T) -> Result<Vec<fs::DirEntry>, MoonError> {
    let path = path.as_ref();
    let handle_error = |e| map_io_to_fs_error(e, path.to_path_buf());

    let entries = fs::read_dir(path).map_err(handle_error)?;
    let mut results = vec![];

    for entry in entries {
        match entry {
            Ok(dir) => {
                results.push(dir);
            }
            Err(e) => return Err(handle_error(e)),
        }
    }

    Ok(results)
}

#[inline]
pub fn read_dir_all<T: AsRef<Path> + Send>(path: T) -> Result<Vec<fs::DirEntry>, MoonError> {
    let entries = read_dir(path)?;
    let mut results = vec![];

    for entry in entries {
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                results.extend(read_dir_all(&entry.path())?);
            } else {
                results.push(entry);
            }
        }
    }

    Ok(results)
}

#[inline]
pub fn read<T: AsRef<Path>>(path: T) -> Result<String, MoonError> {
    let path = path.as_ref();
    let data = fs::read_to_string(path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(data)
}

#[inline]
pub fn remove<T: AsRef<Path>>(path: T) -> Result<(), MoonError> {
    let path = path.as_ref();

    if path.is_file() {
        remove_file(path)?;
    } else if path.is_dir() {
        remove_dir_all(path)?;
    }

    Ok(())
}

#[inline]
pub fn remove_file<T: AsRef<Path>>(path: T) -> Result<(), MoonError> {
    let path = path.as_ref();

    if path.exists() {
        fs::remove_file(&path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

#[inline]
pub fn remove_dir_all<T: AsRef<Path>>(path: T) -> Result<(), MoonError> {
    let path = path.as_ref();

    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;
    }

    Ok(())
}

pub type RemoveDirContentsResult = (usize, u64);

pub fn remove_dir_stale_contents<P: AsRef<Path>>(
    dir: P,
    duration: Duration,
) -> Result<RemoveDirContentsResult, MoonError> {
    let mut files_deleted: usize = 0;
    let mut bytes_saved: u64 = 0;
    let threshold = SystemTime::now() - duration;

    for entry in read_dir(dir.as_ref())? {
        let path = entry.path();

        if path.is_file() {
            let mut bytes = 0;

            if let Ok(metadata) = entry.metadata() {
                bytes = metadata.len();

                if let Ok(filetime) = metadata.accessed().or_else(|_| metadata.created()) {
                    if filetime > threshold {
                        // Not stale yet
                        continue;
                    }
                } else {
                    // Not supported in environment
                    continue;
                }
            }

            if remove_file(path).is_ok() {
                files_deleted += 1;
                bytes_saved += bytes;
            }
        }
    }

    Ok((files_deleted, bytes_saved))
}

#[inline]
pub fn rename<F: AsRef<Path>, T: AsRef<Path>>(from: F, to: T) -> Result<(), MoonError> {
    let from = from.as_ref();

    fs::rename(from, to.as_ref()).map_err(|e| map_io_to_fs_error(e, from.to_path_buf()))?;

    Ok(())
}

#[inline]
pub fn write<T: AsRef<Path>>(path: T, data: impl AsRef<[u8]>) -> Result<(), MoonError> {
    let path = path.as_ref();

    fs::write(path, data).map_err(|e| map_io_to_fs_error(e, path.to_path_buf()))?;

    Ok(())
}

pub mod temp {
    use super::*;
    use moon_constants::CONFIG_DIRNAME;

    pub fn get_dir() -> PathBuf {
        crate::get_workspace_root()
            .join(CONFIG_DIRNAME)
            .join("cache")
            .join("temp")
    }

    pub fn get_file(source: &str, ext: &str) -> PathBuf {
        get_dir().join(format!("{:x}.{}", md5::compute(source), ext))
    }

    pub fn read<P: AsRef<Path>>(path: P) -> Result<Option<String>, MoonError> {
        let file = path.as_ref();

        if !file.exists() {
            return Ok(None);
        }

        // Temp files only last for 4 hours (half a workday)
        let threshold = SystemTime::now() - Duration::from_secs(60 * 60 * 4);

        if let Ok(metadata) = file.metadata() {
            if let Ok(filetime) = metadata.created() {
                if filetime > threshold {
                    remove_file(file)?;

                    return Ok(None);
                }
            }
        }

        Ok(Some(super::read(file)?))
    }

    pub fn write<P: AsRef<Path>, D: AsRef<str>>(path: P, data: D) -> Result<(), MoonError> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        super::write(path, data.as_ref())?;

        Ok(())
    }
}
