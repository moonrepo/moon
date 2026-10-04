use moon_common::cacheable;
use moon_config::{
    Input, MergeStrategy, TaskOperatingSystem, TaskOptionAffectedFilesPattern, TaskOptionCache,
    TaskOptionEnvOverride, TaskOptionRunInCI, TaskOutputStyle, TaskPriority, TaskUnixShell,
    TaskWindowsShell,
};
use std::fmt;

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskOptionAffectedFiles {
        pub filter: Vec<String>,
        pub ignore_project_boundary: bool,
        pub pass: TaskOptionAffectedFilesPattern,
        pub pass_inputs_when_no_match: bool,
        pub pass_dot_when_no_results: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskOptions {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub affected_files: Option<TaskOptionAffectedFiles>,

        pub allow_failure: bool,

        pub cache: TaskOptionCache,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_key: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_lifetime: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub env_files: Option<Vec<Input>>,

        pub env_override: TaskOptionEnvOverride,

        pub expect_failure: bool,

        pub infer_inputs: bool,

        pub internal: bool,

        pub interactive: bool,

        pub merge_args: MergeStrategy,

        pub merge_checks: MergeStrategy,

        pub merge_deps: MergeStrategy,

        pub merge_env: MergeStrategy,

        pub merge_inputs: MergeStrategy,

        pub merge_outputs: MergeStrategy,

        pub merge_tags: MergeStrategy,

        pub merge_toolchains: MergeStrategy,

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

        pub run_in_sync_phase: bool,

        pub run_from_workspace_root: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub shell: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub timeout: Option<u64>,

        pub unix_shell: TaskUnixShell,

        pub windows_shell: TaskWindowsShell,
    }
);

impl Default for TaskOptions {
    fn default() -> Self {
        TaskOptions {
            affected_files: None,
            allow_failure: false,
            cache: TaskOptionCache::Enabled(true),
            cache_key: None,
            cache_lifetime: None,
            env_files: None,
            env_override: TaskOptionEnvOverride::default(),
            expect_failure: false,
            infer_inputs: false,
            internal: false,
            interactive: false,
            merge_args: MergeStrategy::Append,
            merge_checks: MergeStrategy::Append,
            merge_deps: MergeStrategy::Append,
            merge_env: MergeStrategy::Append,
            merge_inputs: MergeStrategy::Append,
            merge_outputs: MergeStrategy::Append,
            merge_tags: MergeStrategy::Append,
            merge_toolchains: MergeStrategy::Append,
            mutex: None,
            os: None,
            output_style: None,
            persistent: false,
            priority: TaskPriority::Normal,
            retry_count: 0,
            run_deps_in_parallel: true,
            run_in_sync_phase: false,
            run_from_workspace_root: false,
            run_in_ci: TaskOptionRunInCI::Enabled(true),
            shell: Some(true),
            timeout: None,
            unix_shell: TaskUnixShell::Bash,
            windows_shell: TaskWindowsShell::Pwsh,
        }
    }
}

/// How the exit code of a task with `expectFailure` relates to that expectation.
/// Only a normal failure satisfies it, as commands that could not run, or were
/// killed by a signal, are not the failure that was expected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskExpectedFailure {
    /// Failed with an exit code that satisfies the expectation.
    Failed,
    /// Passed with exit code 0.
    Passed,
    /// Failed with exit code 126.
    NotExecutable,
    /// Failed with exit code 127.
    NotFound,
    /// Killed by a signal, reported by a shell as 128 + signal.
    Signaled(i32),
    /// Exited with a code outside of 0-255, like a crash on Windows.
    Abnormal(i32),
}

impl TaskExpectedFailure {
    pub fn from_exit_code(code: i32) -> Self {
        match code {
            0 => Self::Passed,
            126 => Self::NotExecutable,
            127 => Self::NotFound,
            129..=192 => Self::Signaled(code - 128),
            1..=255 => Self::Failed,
            _ => Self::Abnormal(code),
        }
    }

    pub fn is_expected(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

impl fmt::Display for TaskExpectedFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed => write!(f, "failed as expected"),
            Self::Passed => write!(f, "passed"),
            Self::NotExecutable => write!(f, "exited with code 126 (command not executable)"),
            Self::NotFound => write!(f, "exited with code 127 (command not found)"),
            Self::Signaled(signal) => {
                write!(
                    f,
                    "exited with code {} (killed by signal {signal})",
                    signal + 128
                )
            }
            Self::Abnormal(code) => write!(f, "exited with code {code}"),
        }
    }
}
