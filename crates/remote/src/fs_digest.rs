// Note: Don't use `starbase_utils::fs` as it spams the logs far too much!

use bazel_remote_apis::build::bazel::remote::execution::v2::{
    NodeProperties, OutputFile, OutputSymlink,
};
use starbase_utils::fs::FsError;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

fn apply_node_properties(path: &Path, props: &NodeProperties) -> miette::Result<()> {
    if let Some(mtime) = &props.mtime {
        let modified = Duration::new(mtime.seconds as u64, mtime.nanos as u32);

        let file = fs::File::options()
            .write(true)
            .open(path)
            .map_err(|error| FsError::Write {
                path: path.to_owned(),
                error: Box::new(error),
            })?;

        file.set_modified(UNIX_EPOCH + modified)
            .map_err(|error| FsError::Write {
                path: path.to_owned(),
                error: Box::new(error),
            })?;
    }

    #[cfg(unix)]
    if let Some(mode) = &props.unix_mode {
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(path, fs::Permissions::from_mode(mode.value)).map_err(|error| {
            FsError::Perms {
                path: path.to_path_buf(),
                error: Box::new(error),
            }
        })?;
    }

    Ok(())
}

pub fn write_output_file(
    output_path: &Path,
    bytes: &[u8],
    file: &OutputFile,
) -> miette::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| FsError::Create {
            path: parent.to_path_buf(),
            error: Box::new(error),
        })?;
    }

    fs::write(output_path, bytes).map_err(|error| FsError::Write {
        path: output_path.to_owned(),
        error: Box::new(error),
    })?;

    if let Some(props) = &file.node_properties {
        apply_node_properties(output_path, props)?;
    }

    Ok(())
}

pub fn link_output_file(
    from_path: PathBuf,
    to_path: PathBuf,
    link: &OutputSymlink,
) -> miette::Result<()> {
    if let Some(parent) = to_path.parent() {
        fs::create_dir_all(parent).map_err(|error| FsError::Create {
            path: parent.to_path_buf(),
            error: Box::new(error),
        })?;
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
        apply_node_properties(&to_path, props)?;
    }

    Ok(())
}
