use crate::errors::ProjectError;
use crate::types::{ExpandedFiles, TouchedFilePaths};
use globset::{Glob, GlobSetBuilder};
use moon_config::{
    FilePath, FilePathOrGlob, TargetID, TaskConfig, TaskMergeStrategy, TaskOptionsConfig, TaskType,
};
use moon_logger::{color, debug};
use moon_utils::is_glob;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOptions {
    pub merge_args: TaskMergeStrategy,

    pub merge_deps: TaskMergeStrategy,

    pub merge_inputs: TaskMergeStrategy,

    pub merge_outputs: TaskMergeStrategy,

    pub retry_count: u8,

    pub run_in_ci: bool,

    pub run_from_workspace_root: bool,
}

impl TaskOptions {
    pub fn merge(&mut self, config: &TaskOptionsConfig) {
        if let Some(merge_args) = &config.merge_args {
            self.merge_args = merge_args.clone();
        }

        if let Some(merge_deps) = &config.merge_deps {
            self.merge_deps = merge_deps.clone();
        }

        if let Some(merge_inputs) = &config.merge_inputs {
            self.merge_inputs = merge_inputs.clone();
        }

        if let Some(merge_outputs) = &config.merge_outputs {
            self.merge_outputs = merge_outputs.clone();
        }

        if let Some(retry_count) = &config.retry_count {
            self.retry_count = *retry_count;
        }

        if let Some(run_in_ci) = &config.run_in_ci {
            self.run_in_ci = *run_in_ci;
        }

        if let Some(run_from_workspace_root) = &config.run_from_workspace_root {
            self.run_from_workspace_root = *run_from_workspace_root;
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Task {
    pub args: Vec<String>,

    pub command: String,

    pub deps: Vec<TargetID>,

    pub inputs: Vec<FilePathOrGlob>,

    #[serde(skip)]
    pub input_globs: Vec<FilePathOrGlob>,

    #[serde(skip)]
    pub input_paths: ExpandedFiles,

    pub options: TaskOptions,

    pub outputs: Vec<FilePath>,

    #[serde(skip)]
    pub output_paths: ExpandedFiles,

    pub target: TargetID,

    #[serde(rename = "type")]
    pub type_of: TaskType,
}

impl Task {
    pub fn from_config(target: TargetID, config: &TaskConfig) -> Self {
        let cloned_config = config.clone();
        let cloned_options = cloned_config.options.unwrap_or_default();

        let task = Task {
            args: cloned_config.args.unwrap_or_else(Vec::new),
            command: cloned_config.command.unwrap_or_default(),
            deps: cloned_config.deps.unwrap_or_else(Vec::new),
            inputs: cloned_config.inputs.unwrap_or_else(Vec::new),
            input_globs: vec![],
            input_paths: HashSet::new(),
            options: TaskOptions {
                merge_args: cloned_options.merge_args.unwrap_or_default(),
                merge_deps: cloned_options.merge_deps.unwrap_or_default(),
                merge_inputs: cloned_options.merge_inputs.unwrap_or_default(),
                merge_outputs: cloned_options.merge_outputs.unwrap_or_default(),
                retry_count: cloned_options.retry_count.unwrap_or_default(),
                run_in_ci: cloned_options.run_in_ci.unwrap_or_default(),
                run_from_workspace_root: cloned_options.run_from_workspace_root.unwrap_or_default(),
            },
            outputs: cloned_config.outputs.unwrap_or_else(Vec::new),
            output_paths: HashSet::new(),
            target: target.clone(),
            type_of: cloned_config.type_of.unwrap_or_default(),
        };

        debug!(
            target: "moon:project",
            "Creating task {} for command {}",
            color::id(&target),
            color::shell(&task.command)
        );

        task
    }

    fn expand_io_path(&self, workspace_root: &Path, project_root: &Path, file: &str) -> PathBuf {
        if file.starts_with('/') {
            workspace_root.join(file)
        } else {
            project_root.join(file)
        }
    }

    /// Expand the inputs list to a set of absolute file paths.
    pub fn expand_inputs(
        &mut self,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<(), ProjectError> {
        for file in &self.inputs {
            // Globs are separate from paths as we can't canonicalize it,
            // and we also need strings for `globset`.
            if is_glob(file) {
                self.input_globs.push(file.to_owned());
            } else {
                let file_path = self.expand_io_path(workspace_root, project_root, file);

                self.input_paths
                    .insert(file_path.canonicalize().map_err(|_| {
                        ProjectError::InvalidUtf8File(String::from(file_path.to_string_lossy()))
                    })?);
            }
        }

        Ok(())
    }

    /// Expand the outputs list to a set of absolute file paths.
    pub fn expand_outputs(
        &mut self,
        workspace_root: &Path,
        project_root: &Path,
    ) -> Result<(), ProjectError> {
        for file in &self.outputs {
            if is_glob(file) {
                return Err(ProjectError::NoOutputGlob(
                    file.to_owned(),
                    self.target.clone(),
                ));
            } else {
                let file_path = self.expand_io_path(workspace_root, project_root, file);

                self.output_paths
                    .insert(file_path.canonicalize().map_err(|_| {
                        ProjectError::InvalidUtf8File(String::from(file_path.to_string_lossy()))
                    })?);
            }
        }

        Ok(())
    }

    /// Return true if this task is affected, based on touched files.
    /// Will attempt to find any file that matches our list of inputs.
    pub fn is_affected(
        &self,
        project_root: &Path,
        touched_files: &TouchedFilePaths,
    ) -> Result<bool, ProjectError> {
        // We have nothing to compare against, so treat it as always affected
        if self.inputs.is_empty() {
            return Ok(true);
        }

        let mut glob_builder = GlobSetBuilder::new();

        for glob in &self.input_globs {
            glob_builder.add(Glob::new(glob)?);
        }

        let globs = glob_builder.build()?;

        for file in touched_files {
            // Not located within the parent project, skip it
            if !file.starts_with(project_root) {
                continue;
            }

            if self.input_paths.contains(file) || globs.is_match(file) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn merge(&mut self, config: &TaskConfig) {
        // Merge options first incase the merge strategy has changed
        if let Some(options) = &config.options {
            self.options.merge(options);
        }

        // Then merge the actual task fields
        if let Some(command) = &config.command {
            self.command = command.clone();
        }

        if let Some(args) = &config.args {
            self.args = self.merge_string_vec(&self.args, args, &self.options.merge_args);
        }

        if let Some(deps) = &config.deps {
            self.deps = self.merge_string_vec(&self.deps, deps, &self.options.merge_deps);
        }

        if let Some(inputs) = &config.inputs {
            self.inputs = self.merge_string_vec(&self.inputs, inputs, &self.options.merge_inputs);
        }

        if let Some(outputs) = &config.outputs {
            self.outputs =
                self.merge_string_vec(&self.outputs, outputs, &self.options.merge_outputs);
        }

        if let Some(type_of) = &config.type_of {
            self.type_of = type_of.clone();
        }
    }

    fn merge_string_vec(
        &self,
        base: &[String],
        next: &[String],
        strategy: &TaskMergeStrategy,
    ) -> Vec<String> {
        let mut list: Vec<String> = vec![];

        // This is easier than .extend() as we need to clone the inner string
        let mut merge = |inner_list: &[String]| {
            for item in inner_list {
                list.push(item.clone());
            }
        };

        match strategy {
            TaskMergeStrategy::Append => {
                merge(base);
                merge(next);
            }
            TaskMergeStrategy::Prepend => {
                merge(next);
                merge(base);
            }
            TaskMergeStrategy::Replace => {
                merge(next);
            }
        }

        list
    }
}
