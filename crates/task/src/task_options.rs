use moon_common::cacheable;
use moon_config::{
    Input, MergeStrategy, TaskOperatingSystem, TaskOptionAffectedFilesPattern, TaskOptionCache,
    TaskOptionEnvOverride, TaskOptionRunInCI, TaskOutputStyle, TaskPriority, TaskUnixShell,
    TaskWindowsShell, is_default, is_false, is_true,
};
use std::fmt;

// Fields are not serialized when they're the default value, which must match
// the value in the `Default` implementation, as that's used when deserializing

fn is_cache_enabled(value: &TaskOptionCache) -> bool {
    *value == TaskOptionCache::Enabled(true)
}

fn is_run_in_ci_enabled(value: &TaskOptionRunInCI) -> bool {
    *value == TaskOptionRunInCI::Enabled(true)
}

fn is_shell_enabled(value: &Option<bool>) -> bool {
    *value == Some(true)
}

cacheable!(
    #[derive(Clone, Debug, Default, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskOptionAffectedFiles {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub filter: Vec<String>,

        #[serde(skip_serializing_if = "is_false")]
        pub ignore_project_boundary: bool,

        #[serde(skip_serializing_if = "is_default")]
        pub pass: TaskOptionAffectedFilesPattern,

        #[serde(skip_serializing_if = "is_false")]
        pub pass_inputs_when_no_match: bool,

        #[serde(skip_serializing_if = "is_false")]
        pub pass_dot_when_no_results: bool,
    }
);

cacheable!(
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[serde(default)]
    pub struct TaskOptions {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub affected_files: Option<TaskOptionAffectedFiles>,

        #[serde(skip_serializing_if = "is_false")]
        pub allow_failure: bool,

        #[serde(skip_serializing_if = "is_cache_enabled")]
        pub cache: TaskOptionCache,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_key: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub cache_lifetime: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub env_files: Option<Vec<Input>>,

        #[serde(skip_serializing_if = "is_default")]
        pub env_override: TaskOptionEnvOverride,

        #[serde(skip_serializing_if = "is_false")]
        pub expect_failure: bool,

        #[serde(skip_serializing_if = "is_false")]
        pub infer_inputs: bool,

        #[serde(skip_serializing_if = "is_false")]
        pub internal: bool,

        #[serde(skip_serializing_if = "is_false")]
        pub interactive: bool,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_args: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_checks: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_deps: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_env: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_inputs: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_outputs: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_tags: MergeStrategy,

        #[serde(skip_serializing_if = "is_default")]
        pub merge_toolchains: MergeStrategy,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub mutex: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub os: Option<Vec<TaskOperatingSystem>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub output_style: Option<TaskOutputStyle>,

        #[serde(skip_serializing_if = "is_false")]
        pub persistent: bool,

        #[serde(skip_serializing_if = "is_default")]
        pub priority: TaskPriority,

        #[serde(skip_serializing_if = "is_default")]
        pub retry_count: u8,

        #[serde(skip_serializing_if = "is_true")]
        pub run_deps_in_parallel: bool,

        #[serde(rename = "runInCI", skip_serializing_if = "is_run_in_ci_enabled")]
        pub run_in_ci: TaskOptionRunInCI,

        #[serde(skip_serializing_if = "is_false")]
        pub run_in_sync_phase: bool,

        #[serde(skip_serializing_if = "is_false")]
        pub run_from_workspace_root: bool,

        #[serde(skip_serializing_if = "is_shell_enabled")]
        pub shell: Option<bool>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub timeout: Option<u64>,

        #[serde(skip_serializing_if = "is_default")]
        pub unix_shell: TaskUnixShell,

        #[serde(skip_serializing_if = "is_default")]
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
