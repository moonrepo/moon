use crate::portable_path::FilePath;
use crate::shapes::{InputPath, OneOrMany};
use crate::{config_enum, config_struct, config_unit_enum, generate_switch};
use schematic::schema::{StringType, UnionType};
use schematic::{Config, ConfigEnum, Schema, SchemaBuilder, Schematic, ValidateError};
use std::env::consts;
use std::str::FromStr;

fn validate_interactive<C>(
    enabled: &bool,
    options: &PartialTaskOptionsConfig,
    _ctx: &C,
    _finalize: bool,
) -> Result<(), ValidateError> {
    if *enabled && options.persistent.is_some_and(|v| v) {
        return Err(ValidateError::new(
            "an interactive task cannot be persistent",
        ));
    }

    Ok(())
}

config_enum!(
    /// The pattern in which affected files will be passed to the affected task.
    #[serde(expecting = "expected `args`, `env`, or a boolean")]
    pub enum TaskOptionAffectedFiles {
        /// Passed as command line arguments.
        Args,
        /// Passed as environment variables.
        Env,
        /// Passed as command line arguments and environment variables.
        #[serde(untagged)]
        Enabled(bool),
    }
);

generate_switch!(TaskOptionAffectedFiles, ["args", "env"]);

config_enum!(
    /// The pattern in which a task is dependent on a `.env` file.
    #[serde(
        untagged,
        expecting = "expected a boolean, a file path, or a list of file paths"
    )]
    pub enum TaskOptionEnvFile {
        /// Uses an `.env` file in the project root.
        Enabled(bool),
        /// Explicit path to an `.env` file.
        File(FilePath),
        /// List of explicit `.env` file paths.
        Files(Vec<FilePath>),
    }
);

impl TaskOptionEnvFile {
    pub fn to_input_paths(&self) -> Option<Vec<InputPath>> {
        match self {
            TaskOptionEnvFile::Enabled(true) => Some(vec![InputPath::ProjectFile(".env".into())]),
            TaskOptionEnvFile::Enabled(false) => None,
            TaskOptionEnvFile::File(path) => {
                InputPath::from_str(path.as_str()).ok().map(|p| vec![p])
            }
            TaskOptionEnvFile::Files(paths) => Some(
                paths
                    .iter()
                    .flat_map(|p| InputPath::from_str(p.as_str()).ok())
                    .collect(),
            ),
        }
    }
}

impl Schematic for TaskOptionEnvFile {
    fn schema_name() -> Option<String> {
        Some("TaskOptionEnvFile".into())
    }

    fn build_schema(mut schema: SchemaBuilder) -> Schema {
        schema.union(UnionType::new_any([
            schema.infer::<bool>(),
            schema.infer::<String>(),
            schema.infer::<Vec<String>>(),
        ]))
    }
}

config_enum!(
    /// The pattern in which to run the task automatically in CI.
    #[serde(expecting = "expected `always`, `affected`, or a boolean")]
    pub enum TaskOptionRunInCI {
        /// Always run, regardless of affected.
        Always,
        /// Only run if affected by touched files.
        Affected,
        /// Either affected, or don't run at all.
        #[serde(untagged)]
        Enabled(bool),
    }
);

generate_switch!(TaskOptionRunInCI, ["always", "affected"]);

config_unit_enum!(
    /// The strategy in which to merge a specific task option.
    #[derive(ConfigEnum)]
    pub enum TaskMergeStrategy {
        #[default]
        Append,
        Prepend,
        Preserve,
        Replace,
    }
);

config_unit_enum!(
    /// The style in which task output will be printed to the console.
    #[derive(ConfigEnum)]
    pub enum TaskOutputStyle {
        #[default]
        Buffer,
        BufferOnlyFailure,
        Hash,
        None,
        Stream,
    }
);

config_enum!(
    /// The operating system in which to only run this task on.
    #[derive(ConfigEnum, Copy)]
    pub enum TaskOperatingSystem {
        Linux,
        #[serde(alias = "mac")]
        Macos,
        #[serde(alias = "win")]
        Windows,
    }
);

impl TaskOperatingSystem {
    pub fn is_current_system(&self) -> bool {
        let os = consts::OS;

        match self {
            Self::Linux => os == "linux" || os.ends_with("bsd"),
            Self::Macos => os == "macos",
            Self::Windows => os == "windows",
        }
    }
}

config_unit_enum!(
    /// The priority levels a task can be bucketed into.
    #[derive(ConfigEnum)]
    pub enum TaskPriority {
        Critical = 0,
        High = 1,
        #[default]
        Normal = 2,
        Low = 3,
    }
);

impl TaskPriority {
    pub fn get_level(&self) -> u8 {
        *self as u8
    }
}

config_unit_enum!(
    /// A list of available shells on Unix.
    #[derive(ConfigEnum)]
    pub enum TaskUnixShell {
        #[default]
        Bash,
        Elvish,
        Fish,
        Ion,
        Murex,
        #[serde(alias = "nushell")]
        Nu,
        #[serde(alias = "powershell")]
        Pwsh,
        Xonsh,
        Zsh,
    }
);

config_unit_enum!(
    /// A list of available shells on Windows.
    #[derive(ConfigEnum)]
    pub enum TaskWindowsShell {
        Bash,
        Elvish,
        Fish,
        Murex,
        #[serde(alias = "nushell")]
        Nu,
        #[default]
        #[serde(alias = "powershell")]
        Pwsh,
        Xonsh,
    }
);

config_struct!(
    /// Options to control task inheritance and execution.
    #[derive(Config)]
    pub struct TaskOptionsConfig {
        /// The pattern in which affected files will be passed to the task.
        pub affected_files: Option<TaskOptionAffectedFiles>,

        /// When affected and no files are matching, pass the task inputs
        /// as arguments to the command, instead of `.`.
        pub affected_pass_inputs: Option<bool>,

        /// Allows the task to fail without failing the entire pipeline.
        pub allow_failure: Option<bool>,

        /// Caches the `outputs` of the task. Defaults to `true` if outputs
        /// are configured for the task.
        pub cache: Option<bool>,

        /// A custom key to include in the cache hashing process. Can be
        /// used to invalidate local and remote caches.
        pub cache_key: Option<String>,

        /// Lifetime to cache the task itself, in the format of "1h", "30m", etc.
        /// If not defined, caches live forever, or until inputs change.
        pub cache_lifetime: Option<String>,

        /// Loads and sets environment variables from the `.env` file when
        /// running the task.
        pub env_file: Option<TaskOptionEnvFile>,

        /// Automatically infer inputs from file groups or environment variables
        /// that were utilized within `command`, `script`, `args`, and `env`.
        pub infer_inputs: Option<bool>,

        /// Marks the task as interactive, so that it will run in isolation,
        /// and have direct access to stdin.
        #[setting(validate = validate_interactive)]
        pub interactive: Option<bool>,

        /// Marks the task as internal, which disables it from begin ran
        /// from the command line, but can be depended on.
        pub internal: Option<bool>,

        /// The default strategy to use when merging `args`, `deps`, `env`,
        /// `inputs`, or `outputs` with an inherited task. Can be overridden
        /// with the other field-specific merge options.
        pub merge: Option<TaskMergeStrategy>,

        /// The strategy to use when merging `args` with an inherited task.
        pub merge_args: Option<TaskMergeStrategy>,

        /// The strategy to use when merging `deps` with an inherited task.
        pub merge_deps: Option<TaskMergeStrategy>,

        /// The strategy to use when merging `env` with an inherited task.
        pub merge_env: Option<TaskMergeStrategy>,

        /// The strategy to use when merging `inputs` with an inherited task.
        pub merge_inputs: Option<TaskMergeStrategy>,

        /// The strategy to use when merging `outputs` with an inherited task.
        pub merge_outputs: Option<TaskMergeStrategy>,

        /// Creates an exclusive lock on a virtual resource, preventing other
        /// tasks using the same resource from running concurrently.
        pub mutex: Option<String>,

        /// The operating system in which to only run this task on.
        pub os: Option<OneOrMany<TaskOperatingSystem>>,

        /// The style in which task output will be printed to the console.
        #[setting(env = "MOON_OUTPUT_STYLE")]
        pub output_style: Option<TaskOutputStyle>,

        /// Marks the task as persistent (continuously running). This is ideal
        /// for watchers, servers, or never-ending processes.
        pub persistent: Option<bool>,

        /// Marks the task with a certain priority, which determines the order
        /// in which it is ran within the pipeline.
        pub priority: Option<TaskPriority>,

        /// The number of times a failing task will be retried to succeed.
        #[setting(env = "MOON_RETRY_COUNT")]
        pub retry_count: Option<u8>,

        /// Runs direct task dependencies (via `deps`) in sequential order.
        /// This _does not_ apply to indirect or transient dependencies.
        pub run_deps_in_parallel: Option<bool>,

        /// Whether to run the task in CI or not, when executing `moon ci` or `moon run`.
        #[serde(rename = "runInCI")]
        pub run_in_ci: Option<TaskOptionRunInCI>,

        /// Runs the task from the workspace root, instead of the project root.
        pub run_from_workspace_root: Option<bool>,

        /// Runs the task within a shell. When not defined, runs the task
        /// directly while relying on `PATH` resolution.
        pub shell: Option<bool>,

        /// The maximum time in seconds that a task can run before being cancelled.
        pub timeout: Option<u64>,

        /// The shell to run the task in when on a Unix-based machine.
        pub unix_shell: Option<TaskUnixShell>,

        /// The shell to run the task in when on a Windows machine.
        pub windows_shell: Option<TaskWindowsShell>,
    }
);
