use crate::task_arg::TaskArg;
use crate::task_options::TaskOptions;
use moon_common::{Id, cacheable, path::WorkspaceRelativePathBuf};
use moon_config::{
    EnvMap, Input, Output, TaskDependencyConfig, TaskOptionRunInCI, TaskPreset, TaskType, is_false,
    schematic::RegexSetting,
};
use moon_target::Target;
use rustc_hash::{FxHashMap, FxHashSet};
use starbase_utils::glob::{self, GlobWalkOptions, split_patterns};
use std::fmt;
use std::path::{Path, PathBuf};

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskState {
        // Inputs are using defaults `**/*`
        #[serde(skip_serializing_if = "is_false")]
        pub default_inputs: bool,

        // Inputs were configured explicitly as `[]`
        #[serde(skip_serializing_if = "is_false")]
        pub empty_inputs: bool,

        // Has the task (and parent project) been expanded
        #[serde(skip_serializing_if = "is_false")]
        pub expanded: bool,

        // Is task defined in a root-level project
        #[serde(skip_serializing_if = "is_false")]
        pub root_level: bool,

        // The `runInCI` option was configured explicitly
        #[serde(skip_serializing_if = "is_false")]
        pub set_run_in_ci: bool,

        // Has shell been explicitly disabled
        #[serde(skip_serializing_if = "is_false")]
        pub shell_disabled: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskFileInput {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub content: Option<RegexSetting>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub optional: Option<bool>,
    }
);

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskGlobInput {
        #[serde(skip_serializing_if = "is_false")]
        pub cache: bool,
    }
);

impl Default for TaskGlobInput {
    fn default() -> Self {
        Self { cache: true }
    }
}

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskFileOutput {
        pub optional: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskGlobOutput {}
);

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct Task {
        pub command: TaskArg,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub args: Vec<TaskArg>,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub deps: Vec<TaskDependencyConfig>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub description: Option<String>,

        #[serde(skip_serializing_if = "EnvMap::is_empty")]
        pub env: EnvMap,

        pub id: Id,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub inputs: Vec<Input>,

        #[serde(skip_serializing_if = "FxHashSet::is_empty")]
        pub input_env: FxHashSet<String>,

        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub input_files: FxHashMap<WorkspaceRelativePathBuf, TaskFileInput>,

        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub input_globs: FxHashMap<WorkspaceRelativePathBuf, TaskGlobInput>,

        pub options: TaskOptions,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub outputs: Vec<Output>,

        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub output_files: FxHashMap<WorkspaceRelativePathBuf, TaskFileOutput>,

        #[serde(skip_serializing_if = "FxHashMap::is_empty")]
        pub output_globs: FxHashMap<WorkspaceRelativePathBuf, TaskGlobOutput>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub preset: Option<TaskPreset>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub script: Option<String>,

        pub state: TaskState,

        pub target: Target,

        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub toolchains: Vec<Id>,

        #[serde(rename = "type")]
        pub type_of: TaskType,
    }
);

impl Task {
    /// Create a globset of all input globs to match with.
    pub fn create_globset(&self) -> miette::Result<glob::GlobSet<'_>> {
        // Both inputs/outputs may have a mix of negated and
        // non-negated globs, so we must split them into groups
        let (gi, ni) = split_patterns(self.input_globs.keys());
        let (go, no) = split_patterns(self.output_globs.keys());

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
    /// the provided changed files list.
    pub fn get_affected_files<S: AsRef<str>>(
        &self,
        workspace_root: &Path,
        changed_files: &FxHashSet<WorkspaceRelativePathBuf>,
        project_source: S,
    ) -> miette::Result<Vec<PathBuf>> {
        let mut files = vec![];
        let globset = self.create_globset()?;
        let project_source = project_source.as_ref();

        for file in changed_files {
            // Don't run on files outside of the project
            if file.starts_with(project_source)
                && (self.input_files.contains_key(file) || globset.matches(file.as_str()))
            {
                files.push(file.to_logical_path(workspace_root));
            }
        }

        Ok(files)
    }

    /// Return the task command/args/script as a full command line for
    /// use within logs and debugs.
    pub fn get_command_line(&self) -> String {
        self.script.clone().unwrap_or_else(|| {
            format!(
                "{} {}",
                self.command.get_value(),
                self.args
                    .iter()
                    .map(|arg| arg.get_value())
                    .collect::<Vec<_>>()
                    .join(" ")
            )
        })
    }

    /// Return a list of all workspace-relative input files.
    pub fn get_input_files(&self, workspace_root: &Path) -> miette::Result<Vec<PathBuf>> {
        let mut list = FxHashSet::default();

        for path in self.input_files.keys() {
            let file = path.to_logical_path(workspace_root);

            // Detect if file actually exists
            if file.is_file() && file.exists() {
                list.insert(file);
            }
        }

        list.extend(
            self.get_input_files_with_globs(workspace_root, self.input_globs.iter().collect())?,
        );

        Ok(list.into_iter().collect())
    }

    /// Return a list of all workspace-relative input files based
    /// on the provided globs and their params.
    pub fn get_input_files_with_globs(
        &self,
        workspace_root: &Path,
        globs: FxHashMap<&WorkspaceRelativePathBuf, &TaskGlobInput>,
    ) -> miette::Result<Vec<PathBuf>> {
        let mut list = FxHashSet::default();
        let mut cached_globs = vec![];
        let mut non_cached_globs = vec![];

        for (glob, params) in globs {
            if params.cache {
                cached_globs.push(glob);
            } else {
                non_cached_globs.push(glob);
            }
        }

        if !cached_globs.is_empty() {
            list.extend(glob::walk_fast_with_options(
                workspace_root,
                cached_globs,
                GlobWalkOptions::default().cache().files(),
            )?);
        }

        if !non_cached_globs.is_empty() {
            list.extend(glob::walk_fast_with_options(
                workspace_root,
                non_cached_globs,
                GlobWalkOptions::default().files(),
            )?);
        }

        Ok(list.into_iter().collect())
    }

    /// Return a list of all workspace-relative output files.
    pub fn get_output_files(
        &self,
        workspace_root: &Path,
        include_non_globs: bool,
    ) -> miette::Result<Vec<PathBuf>> {
        let mut list = FxHashSet::default();

        if include_non_globs {
            for file in self.output_files.keys() {
                list.insert(file.to_logical_path(workspace_root));
            }
        }

        if !self.output_globs.is_empty() {
            list.extend(glob::walk_fast_with_options(
                workspace_root,
                self.output_globs.keys(),
                GlobWalkOptions::default().files(),
            )?);
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

    /// Return true if the task is a "no operation" and does nothing.
    pub fn is_no_op(&self) -> bool {
        (self.command == "nop" || self.command == "noop" || self.command == "no-op")
            && self.script.is_none()
    }

    /// Return true if the task is a "run" type.
    pub fn is_run_type(&self) -> bool {
        matches!(self.type_of, TaskType::Run)
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

    /// Return true if the task should run, whether in a CI environment or not.
    pub fn should_run(&self, in_ci: bool) -> bool {
        if in_ci {
            return match self.options.run_in_ci {
                TaskOptionRunInCI::Skip => false,
                TaskOptionRunInCI::Enabled(state) => state,
                _ => true,
            };
        }

        !matches!(self.options.run_in_ci, TaskOptionRunInCI::Only)
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
    fn default() -> Self {
        Self {
            command: TaskArg::new("noop"),
            args: vec![],
            deps: vec![],
            description: None,
            env: EnvMap::default(),
            id: Id::default(),
            inputs: vec![],
            input_env: FxHashSet::default(),
            input_files: FxHashMap::default(),
            input_globs: FxHashMap::default(),
            options: TaskOptions::default(),
            outputs: vec![],
            output_files: FxHashMap::default(),
            output_globs: FxHashMap::default(),
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
