use crate::task_options::TaskOptions;
use moon_common::{
    cacheable,
    path::{ProjectRelativePathBuf, WorkspaceRelativePathBuf},
    Id,
};
use moon_config::{InputPath, OutputPath, PlatformType, TaskDependencyConfig, TaskType};
use moon_target::Target;
use once_cell::sync::OnceCell;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob;
use std::{
    env,
    path::{Path, PathBuf},
};
use tracing::debug;

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct TaskMetadata {
        // Inputs were configured explicitly as `[]`
        pub empty_inputs: bool,

        // Has the task (and parent project) been expanded
        pub expanded: bool,

        // Was configured as a local running task
        pub local_only: bool,

        // Is task defined in a root-level project
        pub root_level: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct Task {
        pub args: Vec<String>,

        pub command: String,

        pub deps: Vec<TaskDependencyConfig>,

        pub description: Option<String>,

        pub env: FxHashMap<String, String>,

        pub id: Id,

        pub inputs: Vec<InputPath>,

        pub input_env: FxHashSet<String>,

        pub input_files: FxHashSet<WorkspaceRelativePathBuf>,

        pub input_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub metadata: TaskMetadata,

        pub options: TaskOptions,

        pub outputs: Vec<OutputPath>,

        pub output_files: FxHashSet<WorkspaceRelativePathBuf>,

        pub output_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub platform: PlatformType,

        pub script: Option<String>,

        pub target: Target,

        #[serde(rename = "type")]
        pub type_of: TaskType,

        #[serde(skip)]
        pub walk_cache: OnceCell<Vec<PathBuf>>,
    }
);

impl Task {
    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> miette::Result<glob::GlobSet> {
        Ok(glob::GlobSet::new_split(
            &self.input_globs,
            &self.output_globs,
        )?)
    }

    /// Return a list of project-relative affected files filtered down from
    /// the provided touched files list.
    pub fn get_affected_files<S: AsRef<str>>(
        &self,
        touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
        project_source: S,
    ) -> miette::Result<Vec<ProjectRelativePathBuf>> {
        let mut files = vec![];
        let globset = self.create_globset()?;
        let project_source = project_source.as_ref();

        for file in touched_files {
            // Don't run on files outside of the project
            if let Ok(project_file) = file.strip_prefix(project_source) {
                if self.input_files.contains(file) || globset.matches(file.as_str()) {
                    files.push(project_file.to_owned());
                }
            }
        }

        Ok(files)
    }

    /// Return the task command/args/script as a full command line for
    /// use within logs and debugs.
    pub fn get_command_line(&self) -> String {
        self.script
            .clone()
            .unwrap_or_else(|| format!("{} {}", self.command, self.args.join(" ")))
    }

    /// Return true if this task is affected based on touched files.
    /// Will attempt to find any file that matches our list of inputs.
    pub fn is_affected(
        &self,
        touched_files: &FxHashSet<WorkspaceRelativePathBuf>,
    ) -> miette::Result<bool> {
        if self.metadata.empty_inputs {
            return Ok(true);
        }

        for var_name in &self.input_env {
            if let Ok(var) = env::var(var_name) {
                if !var.is_empty() {
                    debug!(
                        task = self.target.as_str(),
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
            if self.input_files.contains(file) {
                debug!(
                    task = self.target.as_str(),
                    input = ?file,
                    "Affected by input file",
                );

                return Ok(true);
            }

            if globset.matches(file.as_str()) {
                debug!(
                    task = self.target.as_str(),
                    input = ?file,
                    "Affected by input glob",
                );

                return Ok(true);
            }
        }

        debug!(task = self.target.as_str(), "Not affected by touched files");

        Ok(false)
    }

    /// Return a list of all workspace-relative input files.
    pub fn get_input_files(
        &self,
        workspace_root: &Path,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut list = vec![];

        for path in &self.input_files {
            // Detect if file actually exists
            if path.to_path(workspace_root).is_file() {
                list.push(path.to_owned());
            }
        }

        if !self.input_globs.is_empty() {
            let globs = &self.input_globs;
            let walk_paths = self
                .walk_cache
                .get_or_try_init(|| glob::walk_files(workspace_root, globs))?;

            // Glob results are absolute paths!
            for file in walk_paths {
                list.push(
                    WorkspaceRelativePathBuf::from_path(file.strip_prefix(workspace_root).unwrap())
                        .unwrap(),
                );
            }
        }

        Ok(list)
    }

    /// Return true if the task is a "build" type.
    pub fn is_build_type(&self) -> bool {
        matches!(self.type_of, TaskType::Build) || !self.outputs.is_empty()
    }

    /// Return true if the task has been expanded.
    pub fn is_expanded(&self) -> bool {
        self.metadata.expanded
    }

    /// Return true if an internal task.
    pub fn is_internal(&self) -> bool {
        self.options.internal
    }

    /// Return true if an interactive task.
    pub fn is_interactive(&self) -> bool {
        self.options.interactive
    }

    /// Return true if a local only task.
    pub fn is_local(&self) -> bool {
        self.metadata.local_only
    }

    /// Return true if the task is a "no operation" and does nothing.
    pub fn is_no_op(&self) -> bool {
        (self.command == "nop" || self.command == "noop" || self.command == "no-op")
            && self.script.is_none()
    }

    /// Return true if the task is a "run" type.
    pub fn is_run_type(&self) -> bool {
        matches!(self.type_of, TaskType::Run) || self.is_local()
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
