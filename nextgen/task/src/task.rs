use crate::task_options::TaskOptions;
use moon_common::{
    cacheable,
    path::{ProjectRelativePathBuf, WorkspaceRelativePathBuf},
    Id,
};
use moon_config::{InputPath, OutputPath, PlatformType, TaskType};
use moon_target::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob;
use std::env;
use tracing::debug;

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct TaskFlags {
        // Inputs were configured explicitly as `[]`
        pub empty_inputs: bool,

        // Was configured as a local running task
        pub local: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct Task {
        pub args: Vec<String>,

        pub command: String,

        pub deps: Vec<Target>,

        pub env: FxHashMap<String, String>,

        pub flags: TaskFlags,

        pub id: Id,

        pub inputs: Vec<InputPath>,

        pub input_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub input_paths: FxHashSet<WorkspaceRelativePathBuf>,

        pub input_vars: FxHashSet<String>,

        pub options: TaskOptions,

        pub outputs: Vec<OutputPath>,

        pub output_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub output_paths: FxHashSet<WorkspaceRelativePathBuf>,

        pub platform: PlatformType,

        pub target: Target,

        #[serde(rename = "type")]
        pub type_of: TaskType,
    }
);

impl Task {
    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> Result<glob::GlobSet, glob::GlobError> {
        glob::GlobSet::new_split(&self.input_globs, &self.output_globs)
    }

    /// Return a list of project-relative affected files filtered down from
    /// the provided touched files list.
    pub fn get_affected_files<S: AsRef<str>>(
        &self,
        touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
        project_source: S,
    ) -> Result<Vec<ProjectRelativePathBuf>, glob::GlobError> {
        let mut files = vec![];
        let globset = self.create_globset()?;
        let project_source = project_source.as_ref();

        for file in touched_files {
            // Don't run on files outside of the project
            if let Ok(project_file) = file.strip_prefix(project_source) {
                if self.input_paths.contains(file) || globset.matches(file.as_str()) {
                    files.push(project_file.to_owned());
                }
            }
        }

        Ok(files)
    }

    /// Return true if this task is affected based on touched files.
    /// Will attempt to find any file that matches our list of inputs.
    pub fn is_affected(
        &self,
        touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
    ) -> Result<bool, glob::GlobError> {
        if self.flags.empty_inputs {
            return Ok(true);
        }

        for var_name in &self.input_vars {
            if let Ok(var) = env::var(var_name) {
                if !var.is_empty() {
                    debug!(
                        target = ?self.target,
                        env_key = var_name,
                        env_val = var,
                        "Affected by environment variable",
                    );

                    return Ok(true);
                }
            }
        }

        let globset = self.create_globset()?;

        for file in touched_files {
            if self.input_paths.contains(file) {
                debug!(
                    target = ?self.target,
                    input = ?file,
                    "Affected by input file",
                );

                return Ok(true);
            }

            if globset.matches(file.as_str()) {
                debug!(
                    target = ?self.target,
                    input = ?file,
                    "Affected by input glob",
                );

                return Ok(true);
            }
        }

        debug!(target = ?self.target, "Not affected by touched files");

        Ok(false)
    }

    /// Return true if the task is a "build" type.
    pub fn is_build_type(&self) -> bool {
        matches!(self.type_of, TaskType::Build) || !self.outputs.is_empty()
    }

    /// Return true if the task is a "no operation" and does nothing.
    pub fn is_no_op(&self) -> bool {
        self.command == "nop" || self.command == "noop" || self.command == "no-op"
    }

    /// Return true if the task is a "run" type.
    pub fn is_run_type(&self) -> bool {
        matches!(self.type_of, TaskType::Run) || self.flags.local
    }

    /// Return true if the task is a "test" type.
    pub fn is_test_type(&self) -> bool {
        matches!(self.type_of, TaskType::Test)
    }

    /// Return true if a persistently running task.
    pub fn is_persistent(&self) -> bool {
        self.options.persistent
    }

    /// Return true if the task should run in a CI environment.
    pub fn should_run_in_ci(&self) -> bool {
        if !self.options.run_in_ci {
            return false;
        }

        self.is_build_type() || self.is_test_type()
    }
}
