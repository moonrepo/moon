use crate::fs_digest::*;
use bazel_remote_apis::build::bazel::remote::execution::v2::{
    command, platform, Action, ActionResult, Command, Digest, ExecutedActionMetadata,
};
use miette::IntoDiagnostic;
use moon_action::Operation;
use moon_task::Task;
use std::collections::BTreeMap;
use std::path::Path;

pub struct ActionState<'task> {
    task: &'task Task,

    // RE API
    pub action: Action,
    pub action_result: Option<ActionResult>,
    pub command: Command,

    // To upload
    pub blobs: Vec<Blob>,
}

impl<'task> ActionState<'task> {
    pub fn new(digest: Digest, task: &Task) -> ActionState<'_> {
        let mut action = Action {
            command_digest: Some(digest),
            do_not_cache: !task.options.cache,
            input_root_digest: None, // TODO?
            ..Default::default()
        };

        // https://github.com/bazelbuild/remote-apis/blob/main/build/bazel/remote/execution/v2/platform.md
        if let Some(os_list) = &task.options.os {
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
            arguments: vec![task.command.clone()],
            output_paths: vec![], // TODO
            ..Default::default()
        };

        command.arguments.extend(task.args.clone());

        for (name, value) in BTreeMap::from_iter(task.env.clone()) {
            command
                .environment_variables
                .push(command::EnvironmentVariable { name, value });
        }

        ActionState {
            task,
            action,
            action_result: None,
            command,
            blobs: vec![],
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

        if let Some(exec) = operation.get_output() {
            result.exit_code = exec.exit_code.unwrap_or_default();

            if let Some(stderr) = &exec.stderr {
                let blob = Blob::new(stderr.as_bytes().to_owned());

                result.stderr_digest = Some(blob.digest.clone());
                self.blobs.push(blob);
            }

            if let Some(stdout) = &exec.stdout {
                let blob = Blob::new(stdout.as_bytes().to_owned());

                result.stdout_digest = Some(blob.digest.clone());
                self.blobs.push(blob);
            }
        }

        self.action_result = Some(result);

        Ok(())
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

    pub fn get_command_as_bytes(&self) -> miette::Result<Vec<u8>> {
        bincode::serialize(&self.command).into_diagnostic()
    }

    pub fn prepare_for_upload(&mut self) -> Option<(ActionResult, Vec<Blob>)> {
        self.action_result
            .take()
            .map(|result| (result, self.blobs.drain(0..).collect::<Vec<_>>()))
    }
}
