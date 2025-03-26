// Note: Don't use `starbase_utils::fs` as it spams the logs far too much!

use crate::blob::Blob;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Digest, NodeProperties, OutputDirectory, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_feature_flags::glob_walk_with_options;
use sha2::{Digest as Sha256Digest, Sha256};
use starbase_utils::fs::FsError;
use starbase_utils::glob::GlobWalkOptions;
use std::{
    fs::{self, Metadata},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn create_digest(bytes: &[u8]) -> Digest {
    let mut hasher = Sha256::default();
    hasher.update(bytes);

    Digest {
        hash: format!("{:x}", hasher.finalize()),
        size_bytes: bytes.len() as i64,
    }
}

pub fn create_timestamp(time: SystemTime) -> Option<Timestamp> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| Timestamp {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos() as i32,
        })
}

pub fn create_timestamp_from_naive(time: NaiveDateTime) -> Option<Timestamp> {
    let utc = time.and_utc();

    Some(Timestamp {
        seconds: utc.timestamp(),
        nanos: utc.timestamp_subsec_nanos() as i32,
    })
}

#[cfg(unix)]
fn is_file_executable(_path: &Path, props: &NodeProperties) -> bool {
    props.unix_mode.is_some_and(|mode| mode.value & 0o111 != 0)
}

#[cfg(windows)]
fn is_file_executable(path: &Path, _props: &NodeProperties) -> bool {
    path.extension().is_some_and(|ext| ext == "exe")
}

pub fn compute_node_properties(metadata: &Metadata) -> NodeProperties {
    let mut props = NodeProperties::default();

    if let Ok(time) = metadata.modified() {
        props.mtime = create_timestamp(time);
    }

    #[cfg(unix)]
    {
        use bazel_remote_apis::google::protobuf::UInt32Value;
        use std::os::unix::fs::PermissionsExt;

        props.unix_mode = Some(UInt32Value {
            value: metadata.permissions().mode(),
        });
    }

    props
}

#[derive(Default)]
pub struct OutputDigests {
    pub blobs: Vec<Blob>,
    pub dirs: Vec<OutputDirectory>,
    pub files: Vec<OutputFile>,
    pub symlinks: Vec<OutputSymlink>,
}

impl OutputDigests {
    pub fn insert_relative_path(
        &mut self,
        rel_path: WorkspaceRelativePathBuf,
        workspace_root: &Path,
    ) -> miette::Result<()> {
        self.insert_path(rel_path.to_path(workspace_root), workspace_root)
    }

    pub fn insert_path(&mut self, abs_path: PathBuf, workspace_root: &Path) -> miette::Result<()> {
        // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L1233
        let path_to_string = |inner_path: &Path| {
            let outer_path = inner_path.relative_to(workspace_root).unwrap().to_string();

            if let Some(stripped) = outer_path.strip_prefix('/') {
                stripped.to_owned()
            } else {
                outer_path
            }
        };

        let map_read_error = |error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        };

        if abs_path.is_symlink() {
            let link = fs::read_link(&abs_path).map_err(map_read_error)?;
            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);

            self.symlinks.push(OutputSymlink {
                path: path_to_string(&abs_path),
                target: path_to_string(&link),
                node_properties: Some(props),
            });
        } else if abs_path.is_file() {
            let bytes = fs::read(&abs_path).map_err(map_read_error)?;
            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);
            let blob = Blob::from(bytes);

            self.files.push(OutputFile {
                path: path_to_string(&abs_path),
                digest: Some(blob.digest.clone()),
                is_executable: is_file_executable(&abs_path, &props),
                contents: vec![],
                node_properties: Some(props),
            });

            self.blobs.push(blob);
        } else if abs_path.is_dir() {
            // TODO use the REAPI directory types
            for abs_file in
                glob_walk_with_options(abs_path, ["**/*"], GlobWalkOptions::default().files())?
            {
                self.insert_path(abs_file, workspace_root)?;
            }
        }

        Ok(())
    }
}

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
    output_path: PathBuf,
    bytes: Vec<u8>,
    file: &OutputFile,
) -> miette::Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| FsError::Create {
            path: parent.to_path_buf(),
            error: Box::new(error),
        })?;
    }

    fs::write(&output_path, bytes).map_err(|error| FsError::Write {
        path: output_path.clone(),
        error: Box::new(error),
    })?;

    if let Some(props) = &file.node_properties {
        apply_node_properties(&output_path, props)?;
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
