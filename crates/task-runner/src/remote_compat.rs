use crate::output_tree::OutputTree;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Action, ActionResult, ExecutedActionMetadata, NodeProperties, OutputFile, OutputSymlink,
};
use moon_action::Operation;
use moon_hash::{Blob, Digest};
use moon_remote::{LocalDigestExt, create_timestamp, create_timestamp_from_naive};
use starbase_utils::fs::{self, FsError};
use std::fs::{self as fs_std, File, Metadata};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

pub fn create_action(command_digest: &Digest) -> Action {
    Action {
        command_digest: Some(command_digest.to_remote_digest()),
        ..Default::default()
    }
}

/// Build an `ActionResult` from the operation + collected outputs.
///
/// Returns:
/// - The `ActionResult` itself (digests referenced inline).
/// - `inline_blobs`: blobs whose bytes are already in memory (currently just
///   stderr/stdout). Always small.
/// - `output_digests`: digests of output files. The bytes live in the local
///   CAS (the streaming collection step wrote them there); load on demand
///   via `cas.read_bytes(&digest.hash)` if you need to upload them.
pub fn create_action_result(
    operation: &Operation,
    outputs: OutputTree,
) -> miette::Result<(ActionResult, Vec<Blob>, Vec<Digest>)> {
    let mut inline_blobs = vec![];
    let mut output_digests = vec![];
    let mut result = ActionResult {
        execution_metadata: Some(ExecutedActionMetadata {
            worker: "moon".into(),
            execution_start_timestamp: create_timestamp_from_naive(operation.started_at),
            execution_completed_timestamp: operation
                .finished_at
                .and_then(create_timestamp_from_naive),
            ..Default::default()
        }),
        ..Default::default()
    };

    if let Some(exec) = operation.get_exec_output() {
        result.exit_code = exec.exit_code.unwrap_or_default();

        if let Some(stderr) = &exec.stderr {
            let blob = Blob::from_bytes(stderr.as_bytes().to_owned())?;

            result.stderr_digest = Some(blob.digest.to_remote_digest());
            inline_blobs.push(blob);
        }

        if let Some(stdout) = &exec.stdout {
            let blob = Blob::from_bytes(stdout.as_bytes().to_owned())?;

            result.stdout_digest = Some(blob.digest.to_remote_digest());
            inline_blobs.push(blob);
        }
    }

    for (path, target) in outputs.symlinks {
        let abs_path = path.to_logical_path(&outputs.workspace_root);
        let metadata = fs_std::metadata(&abs_path).map_err(|error| FsError::Read {
            path: abs_path,
            error: Box::new(error),
        })?;

        result.output_symlinks.push(OutputSymlink {
            path: path.to_string(),
            target: target.to_string(),
            node_properties: Some(extract_node_properties(&metadata)),
        });
    }

    for (path, digest) in outputs.files {
        let abs_path = path.to_logical_path(&outputs.workspace_root);
        let metadata = fs_std::metadata(&abs_path).map_err(|error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        })?;

        result.output_files.push(OutputFile {
            path: path.to_string(),
            digest: Some(digest.to_remote_digest()),
            is_executable: is_file_executable(&abs_path, &metadata),
            contents: vec![],
            node_properties: Some(extract_node_properties(&metadata)),
        });

        output_digests.push(digest);
    }

    Ok((result, inline_blobs, output_digests))
}

// This is where moon differs from the Bazel RE API. In Bazel,
// we would serialize + hash the `Action` and `Command` types,
// to create the action blob, and upload that specifically.
//
// But those types do not match how our hashing works, so instead,
// we're uploading the bytes of our internal hash manifests. Which
// is better for debugging as hashes match across the board!
//
// Hopefully this doesn't cause issues!
pub fn create_action_blob(digest: &Digest, bytes: &[u8]) -> Blob {
    Blob::new(digest.clone(), bytes.to_owned())
}

#[cfg(unix)]
pub fn is_file_executable(_path: &Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
pub fn is_file_executable(path: &Path, _metadata: &Metadata) -> bool {
    path.extension().is_some_and(|ext| ext == "exe")
}

pub fn extract_node_properties(metadata: &Metadata) -> NodeProperties {
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
