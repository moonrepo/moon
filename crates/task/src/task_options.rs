use moon_common::cacheable;
use moon_config::{
    InputPath, TaskMergeStrategy, TaskOperatingSystem, TaskOptionAffectedFiles, TaskOptionRunInCI,
    TaskOutputStyle, TaskPriority, TaskUnixShell, TaskWindowsShell,
};

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskOptions {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub affected_files: Option<TaskOptionAffectedFiles>,

        pub affected_pass_inputs: bool,

        pub allow_failure: bool,

        pub cache: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_key: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_lifetime: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub env_files: Option<Vec<InputPath>>,

        pub infer_inputs: bool,

        pub internal: bool,

        pub interactive: bool,

        pub merge_args: TaskMergeStrategy,

        pub merge_deps: TaskMergeStrategy,

        pub merge_env: TaskMergeStrategy,

        pub merge_inputs: TaskMergeStrategy,

        pub merge_outputs: TaskMergeStrategy,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mutex: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub os: Option<Vec<TaskOperatingSystem>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub output_style: Option<TaskOutputStyle>,

        pub persistent: bool,

        pub priority: TaskPriority,

        pub retry_count: u8,

        pub run_deps_in_parallel: bool,

        #[serde(rename = "runInCI")]
        pub run_in_ci: TaskOptionRunInCI,

        pub run_from_workspace_root: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub shell: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub timeout: Option<u64>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub unix_shell: Option<TaskUnixShell>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub windows_shell: Option<TaskWindowsShell>,
    }
);

impl Default for TaskOptions {
    fn default() -> Self {
        TaskOptions {
            affected_files: None,
            affected_pass_inputs: false,
            allow_failure: false,
            cache: true,
            cache_key: None,
            cache_lifetime: None,
            env_files: None,
            infer_inputs: true,
            internal: false,
            interactive: false,
            merge_args: TaskMergeStrategy::Append,
            merge_deps: TaskMergeStrategy::Append,
            merge_env: TaskMergeStrategy::Append,
            merge_inputs: TaskMergeStrategy::Append,
            merge_outputs: TaskMergeStrategy::Append,
            mutex: None,
            os: None,
            output_style: None,
            persistent: false,
            priority: TaskPriority::Normal,
            retry_count: 0,
            run_deps_in_parallel: true,
            run_in_ci: TaskOptionRunInCI::Affected,
            run_from_workspace_root: false,
            shell: None,
            timeout: None,
            unix_shell: None,
            windows_shell: None,
        }
    }
}
