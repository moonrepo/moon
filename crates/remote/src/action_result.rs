use crate::blob::*;
use crate::digest_compat::LocalDigestExt;
use crate::remote_error::RemoteError;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, ExecutedActionMetadata, NodeProperties, OutputFile, OutputSymlink,
};
use bazel_remote_apis::google::protobuf::Timestamp;
use chrono::NaiveDateTime;
use moon_action::Operation;
use moon_common::path::PathExt;
use moon_hash::{Blob, OutputBlobs};
use starbase_utils::fs::FsError;
use starbase_utils::glob::{self, GlobWalkOptions};
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ActionResultBuilder<'a> {
    blobs: Vec<CompressableBlob>,
    result: ActionResult,
    workspace_root: &'a Path,
}

impl<'a> ActionResultBuilder<'a> {
    pub fn new(workspace_root: &'a Path) -> Self {
        Self {
            blobs: Vec::new(),
            result: ActionResult::default(),
            workspace_root,
        }
    }

    pub fn build(self) -> (ActionResult, Vec<CompressableBlob>) {
        (self.result, self.blobs)
    }

    pub fn with_operation(&mut self, operation: &Operation) -> miette::Result<()> {
        self.result.execution_metadata = Some(ExecutedActionMetadata {
            worker: "moon".into(),
            execution_start_timestamp: create_timestamp_from_naive(operation.started_at),
            execution_completed_timestamp: operation
                .finished_at
                .and_then(create_timestamp_from_naive),
            ..Default::default()
        });

        // Extract executions outputs (stdout, stderr)
        if let Some(exec) = operation.get_exec_output() {
            self.result.exit_code = exec.exit_code.unwrap_or_default();

            if let Some(stderr) = &exec.stderr {
                let blob = CompressableBlob::from_bytes(stderr.as_bytes().to_owned())?;

                self.result.stderr_digest = Some(blob.digest.to_remote_digest());
                self.blobs.push(blob);
            }

            if let Some(stdout) = &exec.stdout {
                let blob = CompressableBlob::from_bytes(stdout.as_bytes().to_owned())?;

                self.result.stdout_digest = Some(blob.digest.to_remote_digest());
                self.blobs.push(blob);
            }
        }

        Ok(())
    }

    pub fn with_outputs(&mut self, outputs: OutputBlobs) -> miette::Result<()> {
        for (abs_path, blob) in outputs {
            self.insert_output(abs_path, Some(blob))?;
        }

        Ok(())
    }

    fn insert_output(
        &mut self,
        abs_path: PathBuf,
        source_blob: Option<Blob>,
    ) -> miette::Result<()> {
        let map_read_error = |error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        };

        if abs_path.is_symlink() {
            let link = fs::read_link(&abs_path).map_err(map_read_error)?;

            if !abs_path.starts_with(self.workspace_root) || !link.starts_with(self.workspace_root)
            {
                return Err(RemoteError::OutputSymlinkOutsideOfWorkspace {
                    output: abs_path,
                    target: link,
                }
                .into());
            }

            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);

            self.result.output_symlinks.push(OutputSymlink {
                path: self.convert_path(&abs_path)?,
                target: self.convert_path(&link)?,
                node_properties: Some(props),
            });
        } else if abs_path.is_file() {
            if !abs_path.starts_with(self.workspace_root) {
                return Err(RemoteError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
            }

            let metadata = fs::metadata(&abs_path).map_err(map_read_error)?;
            let props = compute_node_properties(&metadata);
            let blob = match source_blob {
                Some(inner) => CompressableBlob::from_blob(inner)?,
                None => CompressableBlob::from_file(&abs_path)?,
            };

            self.result.output_files.push(OutputFile {
                path: self.convert_path(&abs_path)?,
                digest: Some(blob.digest.to_remote_digest()),
                is_executable: is_file_executable(&abs_path, &props),
                contents: vec![],
                node_properties: Some(props),
            });

            self.blobs.push(blob);
        } else if abs_path.is_dir() {
            if !abs_path.starts_with(self.workspace_root) {
                return Err(RemoteError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
            }

            for abs_file in glob::walk_fast_with_options(
                abs_path,
                ["**/*"],
                GlobWalkOptions::default().files(),
            )? {
                self.insert_output(abs_file, None)?;
            }
        }

        Ok(())
    }

    // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/remote_execution.proto#L1233
    fn convert_path(&self, abs_path: &Path) -> miette::Result<String> {
        let outer_path = abs_path
            .relative_to(self.workspace_root)
            .map_err(|_| RemoteError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_owned(),
            })?
            .to_string();

        Ok(outer_path
            .strip_prefix('/')
            .unwrap_or(&outer_path)
            .to_owned())
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
