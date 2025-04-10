use crate::task_options::TaskOptions;
use moon_common::{
    Id, cacheable,
    path::{PathExt, ProjectRelativePathBuf, WorkspaceRelativePathBuf},
};
use moon_config::{
    InputPath, OutputPath, PlatformType, TaskDependencyConfig, TaskPreset, TaskType,
};
use moon_feature_flags::glob_walk_with_options;
use moon_target::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob::{self, GlobWalkOptions, split_patterns};
use std::fmt;
use std::path::Path;

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    pub struct TaskState {
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
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct Task {
        pub args: Vec<String>,

        pub command: String,

        pub deps: Vec<TaskDependencyConfig>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,

        pub env: FxHashMap<String, String>,

        pub id: Id,

        pub inputs: Vec<InputPath>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub input_env: FxHashSet<String>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub input_files: FxHashSet<WorkspaceRelativePathBuf>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub input_globs: FxHashSet<WorkspaceRelativePathBuf>,

        pub options: TaskOptions,

        pub outputs: Vec<OutputPath>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub output_files: FxHashSet<WorkspaceRelativePathBuf>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub output_globs: FxHashSet<WorkspaceRelativePathBuf>,

        #[deprecated]
        pub platform: PlatformType,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub preset: Option<TaskPreset>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub script: Option<String>,

        pub state: TaskState,

        pub target: Target,

        pub toolchains: Vec<Id>,

        #[serde(rename = "type")]
        pub type_of: TaskType,
    }
);

impl Task {
    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> miette::Result<glob::GlobSet> {
        // Both inputs/outputs may have a mix of negated and
        // non-negated globs, so we must split them into groups
        let (gi, ni) = split_patterns(&self.input_globs);
        let (go, no) = split_patterns(&self.output_globs);

        // We then only match against non-negated inputs
        let g = gi;

        // While output non-negated/negated and negated inputs
        // are all considered negations (inputs and outputs
        // shouldn't overlay)
        let mut n = vec![];
        n.extend(go);
        n.extend(ni);
        n.extend(no);

        Ok(glob::GlobSet::new_split(g, n)?)
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

    /// Return a list of all workspace-relative input files.
    pub fn get_input_files(
        &self,
        workspace_root: &Path,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut list = FxHashSet::default();

        for path in &self.input_files {
            // Detect if file actually exists
            if path.to_path(workspace_root).is_file() {
                list.insert(path.to_owned());
            }
        }

        if !self.input_globs.is_empty() {
            for file in glob_walk_with_options(
                workspace_root,
                &self.input_globs,
                GlobWalkOptions::default().cache().files(),
            )? {
                // Glob results are absolute paths!
                list.insert(file.relative_to(workspace_root).unwrap());
            }
        }

        Ok(list.into_iter().collect())
    }

    /// Return a list of all workspace-relative output files.
    pub fn get_output_files(
        &self,
        workspace_root: &Path,
        include_non_globs: bool,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut list = FxHashSet::default();

        if include_non_globs {
            list.extend(self.output_files.clone());
        }

        if !self.output_globs.is_empty() {
            for file in glob_walk_with_options(
                workspace_root,
                &self.output_globs,
                GlobWalkOptions::default().cache().files(),
            )? {
                // Glob results are absolute paths!
                list.insert(file.relative_to(workspace_root).unwrap());
            }
        }

        Ok(list.into_iter().collect())
    }

    /// Return true if the task is a "build" type.
    pub fn is_build_type(&self) -> bool {
        matches!(self.type_of, TaskType::Build) || !self.outputs.is_empty()
    }

    /// Return true if the task has been expanded.
    pub fn is_expanded(&self) -> bool {
        self.state.expanded
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
        self.state.local_only
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

    /// Return true of the task will run in the system toolchain.
    pub fn is_system_toolchain(&self) -> bool {
        self.toolchains.is_empty() || self.toolchains.len() == 1 && self.toolchains[0] == "system"
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
        if !self.options.run_in_ci.is_enabled() {
            return false;
        }

        self.is_build_type() || self.is_test_type()
    }

    /// Convert the task into a fragment.
    pub fn to_fragment(&self) -> TaskFragment {
        TaskFragment {
            target: self.target.clone(),
            toolchains: self.toolchains.clone(),
        }
    }
}

impl Default for Task {
    #[allow(deprecated)]
    fn default() -> Self {
        Self {
            args: vec![],
            command: String::from("noop"),
            deps: vec![],
            description: None,
            env: FxHashMap::default(),
            id: Id::default(),
            inputs: vec![],
            input_env: FxHashSet::default(),
            input_files: FxHashSet::default(),
            input_globs: FxHashSet::default(),
            options: TaskOptions::default(),
            outputs: vec![],
            output_files: FxHashSet::default(),
            output_globs: FxHashSet::default(),
            platform: PlatformType::default(),
            preset: None,
            script: None,
            state: TaskState::default(),
            target: Target::default(),
            toolchains: vec![Id::raw("system")],
            type_of: TaskType::default(),
        }
    }
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.target)
    }
}

cacheable!(
    /// Fragment of a task including important fields.
    #[derive(Clone, Debug, Default, PartialEq)]
    pub struct TaskFragment {
        /// Target of the task.
        pub target: Target,

        /// Toolchains the task belongs to.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub toolchains: Vec<Id>,
    }
);
