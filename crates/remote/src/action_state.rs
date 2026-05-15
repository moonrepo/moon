use crate::blob::*;
use crate::digest_compat::LocalDigestExt;
use crate::fs_digest::{OutputDigests, create_timestamp_from_naive};
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    ActionResult, ExecutedActionMetadata,
};
use moon_action::Operation;
use moon_hash::Digest;
use moon_task::Task;
use std::path::Path;

pub struct ActionState<'task> {
    task: &'task Task,

    // RE API
    pub action_result: Option<ActionResult>,
    pub digest: Digest,

    // Outputs to upload
    pub blobs: Vec<CompressableBlob>,

    // Bytes of our hashed manifest
    pub bytes: Vec<u8>,
}

impl ActionState<'_> {
    pub fn new(digest: Digest, task: &Task) -> ActionState<'_> {
        ActionState {
            task,
            action_result: None,
            digest,
            blobs: vec![],
            bytes: vec![],
        }
    }

    pub fn create_action_result_from_operation(
        &mut self,
        operation: &Operation,
    ) -> miette::Result<()> {
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
                let blob = CompressableBlob::from_bytes(stderr.as_bytes().to_owned())?;

                result.stderr_digest = Some(blob.digest.to_remote_digest());
                self.blobs.push(blob);
            }

            if let Some(stdout) = &exec.stdout {
                let blob = CompressableBlob::from_bytes(stdout.as_bytes().to_owned())?;

                result.stdout_digest = Some(blob.digest.to_remote_digest());
                self.blobs.push(blob);
            }
        }

        self.action_result = Some(result);

        Ok(())
    }

    pub fn set_action_result(&mut self, result: ActionResult) {
        self.action_result = Some(result);
    }

    pub fn compute_outputs(&mut self, workspace_root: &Path) -> miette::Result<()> {
        if let Some(result) = &mut self.action_result {
            let mut outputs = OutputDigests::default();

            for path in self.task.get_output_files(workspace_root, true)? {
                outputs.insert_path(path, workspace_root)?;
            }

            result.output_files = outputs.files;
            result.output_symlinks = outputs.symlinks;
            result.output_directories = outputs.dirs;

            self.blobs.extend(outputs.blobs);
        }

        Ok(())
    }

    pub fn extract_for_upload(&mut self) -> Option<(ActionResult, Vec<CompressableBlob>)> {
        self.action_result
            .take()
            .map(|result| (result, self.blobs.drain(0..).collect::<Vec<_>>()))
    }
}
