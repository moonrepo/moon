use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Digest, NodeProperties, OutputDirectory, OutputFile, OutputSymlink,
};
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use prost_types::Timestamp;
use sha2::{Digest as Sha256Digest, Sha256};
use starbase_utils::fs::{self, FsError};
use std::{
    fs::Metadata,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::warn;

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::default();
    hasher.update(bytes);

    format!("{:x}", hasher.finalize())
}

pub fn create_digest(bytes: &[u8]) -> Digest {
    Digest {
        hash: hash_bytes(bytes),
        size_bytes: bytes.len() as i64,
    }
}

pub fn create_digest_from_path(path: &Path) -> miette::Result<Digest> {
    let bytes = fs::read_file_bytes(path)?;

    Ok(create_digest(&bytes))
}

pub fn create_timestamp(time: SystemTime) -> Timestamp {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();

    Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.as_nanos() as i32,
    }
}

#[cfg(unix)]
fn is_file_executable(_path: &Path, props: &NodeProperties) -> bool {
    props.unix_mode.is_some_and(|mode| mode & 0o111 != 0)
}

#[cfg(windows)]
fn is_file_executable(path: &Path, _props: &NodeProperties) -> bool {
    path.extension().is_some_and(|ext| ext == "exe")
}

pub fn calculate_node_properties(metadata: &Metadata) -> NodeProperties {
    let mut props = NodeProperties::default();

    if let Ok(time) = metadata.modified() {
        props.mtime = Some(create_timestamp(time));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        props.unix_mode = Some(metadata.permissions().mode());
    }

    props
}

#[derive(Default)]
pub struct OutputDigests {
    pub dirs: Vec<OutputDirectory>,
    pub files: Vec<OutputFile>,
    pub symlinks: Vec<OutputSymlink>,
}

pub fn calculate_digests_for_outputs(
    paths: Vec<WorkspaceRelativePathBuf>,
    workspace_root: &Path,
) -> miette::Result<OutputDigests> {
    let mut result = OutputDigests::default();

    for path in paths {
        let abs_path = path.to_path(workspace_root);

        if abs_path.is_file() {
            let metadata = fs::metadata(&abs_path)?;
            let node_properties = calculate_node_properties(&metadata);

            if abs_path.is_symlink() {
                let link = std::fs::read_link(&abs_path).map_err(|error| FsError::Read {
                    path: abs_path.clone(),
                    error: Box::new(error),
                })?;

                result.symlinks.push(OutputSymlink {
                    path: path.to_string(),
                    target: link.relative_to(workspace_root).unwrap().to_string(),
                    node_properties: Some(node_properties),
                });
            } else {
                result.files.push(OutputFile {
                    path: path.to_string(),
                    digest: Some(create_digest_from_path(&abs_path)?),
                    is_executable: is_file_executable(&abs_path, &node_properties),
                    contents: vec![],
                    node_properties: Some(node_properties),
                });
            }
        } else if abs_path.is_dir() {
            warn!(
                dir = ?abs_path,
                "Directories are currently not supported as outputs for remote caching",
            );
        }
    }

    Ok(result)
}
