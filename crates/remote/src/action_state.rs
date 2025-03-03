use crate::blob::*;
use crate::fs_digest::{OutputDigests, create_timestamp_from_naive};
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    Action, ActionResult, Command, Digest, ExecutedActionMetadata, command, platform,
};
use moon_action::Operation;
use moon_task::Task;
use std::collections::BTreeMap;
use std::path::Path;

pub struct ActionState<'task> {
    task: &'task Task,

    // RE API
    pub action: Option<Action>,
    pub action_result: Option<ActionResult>,
    pub command: Option<Command>,
    pub digest: Digest,

    // Outputs to upload
    pub blobs: Vec<Blob>,

    // Bytes of our hashed manifest
    pub bytes: Vec<u8>,
}

impl ActionState<'_> {
    pub fn new(digest: Digest, task: &Task) -> ActionState<'_> {
        ActionState {
            task,
            action: None,
            action_result: None,
            command: None,
            digest,
            blobs: vec![],
            bytes: vec![],
        }
    }

    pub fn create_action_from_task(&mut self) {
        let mut action = Action {
            command_digest: Some(self.digest.clone()),
            do_not_cache: !self.task.options.cache,
            input_root_digest: None, // TODO?
            ..Default::default()
        };

        // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/platform.md
        if let Some(os_list) = &self.task.options.os {
            let platform = action.platform.get_or_insert_default();

            for os in os_list {
                platform.properties.push(platform::Property {
                    name: "OSFamily".into(),
                    value: os.to_string(),
                });
            }
        }

        // Since we don't support (or plan to) remote execution,
        // then we can ignore all the working directory logic
        let mut command = Command {
            arguments: vec![self.task.command.clone()],
            output_paths: vec![], // TODO?
            ..Default::default()
        };

        command.arguments.extend(self.task.args.clone());

        for (name, value) in BTreeMap::from_iter(self.task.env.clone()) {
            command
                .environment_variables
                .push(command::EnvironmentVariable { name, value });
        }

        self.action = Some(action);
        self.command = Some(command);
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
                let blob = Blob::from(stderr.as_bytes().to_owned());

                result.stderr_digest = Some(blob.digest.clone());
                self.blobs.push(blob);
            }

            if let Some(stdout) = &exec.stdout {
                let blob = Blob::from(stdout.as_bytes().to_owned());

                result.stdout_digest = Some(blob.digest.clone());
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
        let mut outputs = OutputDigests::default();

        for path in self.task.get_output_files(workspace_root, true)? {
            outputs.insert_relative_path(path, workspace_root)?;
        }

        if let Some(result) = &mut self.action_result {
            result.output_files = outputs.files;
            result.output_symlinks = outputs.symlinks;
            result.output_directories = outputs.dirs;
            self.blobs.extend(outputs.blobs);
        }

        Ok(())
    }

    pub fn extract_for_upload(&mut self) -> Option<(ActionResult, Vec<Blob>)> {
        self.action_result
            .take()
            .map(|result| (result, self.blobs.drain(0..).collect::<Vec<_>>()))
    }
}
