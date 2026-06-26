use bazel_remote_apis::build::bazel::remote::execution::v2::{
    NodeProperties, OutputFile, OutputSymlink,
};
use starbase_utils::fs::{self, FsError};
use std::fs::{self as fs_std, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

pub fn apply_node_properties(fd: &mut File, props: &NodeProperties) -> std::io::Result<()> {
    if let Some(mtime) = &props.mtime {
        let modified = Duration::new(mtime.seconds as u64, mtime.nanos as u32);

        fd.set_modified(UNIX_EPOCH + modified)?;
    }

    #[cfg(unix)]
    if let Some(mode) = &props.unix_mode {
        use std::os::unix::fs::PermissionsExt;

        fd.set_permissions(fs_std::Permissions::from_mode(mode.value))?;
    }

    Ok(())
}

pub fn write_output_file(
    output_path: PathBuf,
    bytes: &[u8],
    file: &OutputFile,
) -> miette::Result<()> {
    let map_error = |error| FsError::Write {
        path: output_path.clone(),
        error: Box::new(error),
    };

    let mut fd = fs::create_file(&output_path)?;

    fd.write_all(bytes).map_err(map_error)?;

    if let Some(props) = &file.node_properties {
        apply_node_properties(&mut fd, props).map_err(map_error)?;
    }

    Ok(())
}

/// Re-apply an output file's recorded node properties (mtime, permissions) to
/// a file already on disk. Used after hydrating via reflink, where the bytes
/// are cloned from the CAS but the original metadata must still be restored.
pub fn apply_output_file_properties(output_path: &Path, file: &OutputFile) -> miette::Result<()> {
    if let Some(props) = &file.node_properties {
        let mut fd = fs::open_file_for_writing(output_path)?;

        apply_node_properties(&mut fd, props).map_err(|error| FsError::Write {
            path: output_path.to_path_buf(),
            error: Box::new(error),
        })?;
    }

    Ok(())
}

pub fn link_output_file(
    from_path: PathBuf,
    to_path: PathBuf,
    link: &OutputSymlink,
) -> miette::Result<()> {
    if let Some(parent) = to_path.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(windows)]
    {
        if from_path.is_dir() {
            std::os::windows::fs::symlink_dir(&from_path, &to_path).map_err(|error| {
                FsError::Create {
                    path: to_path.clone(),
                    error: Box::new(error),
                }
            })?;
        } else {
            std::os::windows::fs::symlink_file(&from_path, &to_path).map_err(|error| {
                FsError::Create {
                    path: to_path.clone(),
                    error: Box::new(error),
                }
            })?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&from_path, &to_path).map_err(|error| FsError::Create {
            path: to_path.clone(),
            error: Box::new(error),
        })?;
    }

    if let Some(props) = &link.node_properties {
        let map_error = |error| FsError::Write {
            path: to_path.clone(),
            error: Box::new(error),
        };

        let mut fd = fs::open_file_for_writing(&to_path)?;

        apply_node_properties(&mut fd, props).map_err(map_error)?;
    }

    Ok(())
}
