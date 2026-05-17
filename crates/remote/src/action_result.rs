use crate::blob::*;
use crate::digest_compat::LocalDigestExt;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, Digest, ExecutedActionMetadata, NodeProperties, OutputDirectory, OutputFile,
    OutputSymlink,
};
use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use moon_action::Operation;
use moon_common::path::PathExt;
use moon_hash::OutputDigests;
use moon_task::Task;
use starbase_utils::fs::FsError;
use starbase_utils::glob::{self, GlobWalkOptions};
use std::fs::{self, Metadata};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn create_action_result_for_upload(
    operation: &Operation,
    outputs: OutputDigests,
    workspace_root: &Path,
) -> miette::Result<ActionResult> {
    let mut blobs = vec![];
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

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L1233
    let path_to_string = |inner_path: &Path| {
        let outer_path = inner_path
            .relative_to(workspace_root)
            .expect("Output path is outside of the workspace!")
            .to_string();

        if let Some(stripped) = outer_path.strip_prefix('/') {
            stripped.to_owned()
        } else {
            outer_path
        }
    };

    // Extract executions outputs (stdout, stderr)
    if let Some(exec) = operation.get_exec_output() {
        result.exit_code = exec.exit_code.unwrap_or_default();

        if let Some(stderr) = &exec.stderr {
            let blob = CompressableBlob::from_bytes(stderr.as_bytes().to_owned())?;

            result.stderr_digest = Some(blob.digest.to_remote_digest());
            blobs.push(blob);
        }

        if let Some(stdout) = &exec.stdout {
            let blob = CompressableBlob::from_bytes(stdout.as_bytes().to_owned())?;

            result.stdout_digest = Some(blob.digest.to_remote_digest());
            blobs.push(blob);
        }
    }

    // Extract file outputs
    for (abs_path, blob) in outputs.blobs {
        let map_read_error = |error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        };

        if abs_path.is_symlink() {
            let link = fs::read_link(&abs_path).map_err(map_read_error)?;
            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);

            if !abs_path.starts_with(workspace_root) || !link.starts_with(workspace_root) {
                return Err(RemoteError::OutputSymlinkOutsideOfWorkspace {
                    output: abs_path,
                    target: link,
                }
                .into());
            }

            result.output_symlinks.push(OutputSymlink {
                path: path_to_string(&abs_path),
                target: path_to_string(&link),
                node_properties: Some(props),
            });
        } else if abs_path.is_file() {
            let bytes = fs::read(&abs_path).map_err(map_read_error)?;
            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);
            let blob = CompressableBlob::from_bytes(bytes)?;

            result.output_files.push(OutputFile {
                path: path_to_string(&abs_path),
                digest: Some(blob.digest.to_remote_digest()),
                is_executable: is_file_executable(&abs_path, &props),
                contents: vec![],
                node_properties: Some(props),
            });

            blobs.push(blob);
        } else if abs_path.is_dir() {
            for abs_file in glob::walk_fast_with_options(
                abs_path,
                ["**/*"],
                GlobWalkOptions::default().files(),
            )? {
                // TODO
                // self.insert_path(abs_file, workspace_root)?;
            }
        }
    }

    Ok(result)
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

fn compute_node_properties(metadata: &Metadata) -> NodeProperties {
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
