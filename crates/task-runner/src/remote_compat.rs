use crate::output_tree::OutputTree;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Action, ActionResult, ExecutedActionMetadata, OutputFile, OutputSymlink,
};
use moon_action::Operation;
use moon_hash::{Blob, Digest};
use moon_remote::{
    LocalDigestExt, compute_node_properties, create_timestamp_from_naive, is_file_executable,
};
use starbase_utils::fs::FsError;
use std::fs;

pub fn create_action(command_digest: &Digest) -> Action {
    Action {
        command_digest: Some(command_digest.to_remote_digest()),
        ..Default::default()
    }
}

pub fn create_action_result(
    operation: &Operation,
    outputs: OutputTree,
) -> miette::Result<(ActionResult, Vec<Blob>)> {
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

    if let Some(exec) = operation.get_exec_output() {
        result.exit_code = exec.exit_code.unwrap_or_default();

        if let Some(stderr) = &exec.stderr {
            let blob = Blob::from_bytes(stderr.as_bytes().to_owned())?;

            result.stderr_digest = Some(blob.digest.to_remote_digest());
            blobs.push(blob);
        }

        if let Some(stdout) = &exec.stdout {
            let blob = Blob::from_bytes(stdout.as_bytes().to_owned())?;

            result.stdout_digest = Some(blob.digest.to_remote_digest());
            blobs.push(blob);
        }
    }

    for (path, target) in outputs.symlinks {
        let abs_path = path.to_logical_path(&outputs.workspace_root);
        let metadata = fs::metadata(&abs_path).map_err(|error| FsError::Read {
            path: abs_path,
            error: Box::new(error),
        })?;

        result.output_symlinks.push(OutputSymlink {
            path: path.to_string(),
            target: target.to_string(),
            node_properties: Some(compute_node_properties(&metadata)),
        });
    }

    for (path, blob) in outputs.files {
        let abs_path = path.to_logical_path(&outputs.workspace_root);
        let metadata = fs::metadata(&abs_path).map_err(|error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        })?;

        result.output_files.push(OutputFile {
            path: path.to_string(),
            digest: Some(blob.digest.to_remote_digest()),
            is_executable: is_file_executable(&abs_path, &metadata),
            contents: vec![],
            node_properties: Some(compute_node_properties(&metadata)),
        });

        blobs.push(blob);
    }

    Ok((result, blobs))
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
    Blob {
        digest: digest.clone(),
        bytes: bytes.to_owned(),
    }
}
