use moon_common::cacheable;
use moon_config::{
    InputPath, TaskMergeStrategy, TaskOptionAffectedFiles, TaskOutputStyle, TaskUnixShell,
    TaskWindowsShell,
};

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct TaskOptions {
        pub affected_files: Option<TaskOptionAffectedFiles>,

        pub allow_failure: bool,

        pub cache: bool,

        pub env_file: Option<InputPath>,

        pub interactive: bool,

        pub merge_args: TaskMergeStrategy,

        pub merge_deps: TaskMergeStrategy,

        pub merge_env: TaskMergeStrategy,

        pub merge_inputs: TaskMergeStrategy,

        pub merge_outputs: TaskMergeStrategy,

        pub output_style: Option<TaskOutputStyle>,

        pub persistent: bool,

        pub retry_count: u8,

        pub run_deps_in_parallel: bool,

        #[serde(rename = "runInCI")]
        pub run_in_ci: bool,

        pub run_from_workspace_root: bool,

        pub shell: Option<bool>,

        pub unix_shell: TaskUnixShell,

        pub windows_shell: TaskWindowsShell,
    }
);

impl Default for TaskOptions {
    fn default() -> Self {
        TaskOptions {
            affected_files: None,
            allow_failure: false,
            cache: true,
            env_file: None,
            interactive: false,
            merge_args: TaskMergeStrategy::Append,
            merge_deps: TaskMergeStrategy::Append,
            merge_env: TaskMergeStrategy::Append,
            merge_inputs: TaskMergeStrategy::Append,
            merge_outputs: TaskMergeStrategy::Append,
            output_style: None,
            persistent: false,
            retry_count: 0,
            run_deps_in_parallel: true,
            run_in_ci: true,
            run_from_workspace_root: false,
            shell: None,
            unix_shell: TaskUnixShell::System,
            windows_shell: TaskWindowsShell::System,
        }
    }
}
