use crate::portable_path::FilePath;
use crate::shapes::InputPath;
use moon_common::cacheable;
use schematic::schema::StringType;
use schematic::{derive_enum, Config, ConfigEnum, SchemaType, Schematic, ValidateError};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_yaml::Value;
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

/// The pattern in which affected files will be passed to the affected task.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum TaskOptionAffectedFiles {
    /// Passed as command line arguments.
    Args,
    /// Passed as environment variables.
    Env,
    /// Passed as command line arguments and environment variables.
    Enabled(bool),
}

impl Schematic for TaskOptionAffectedFiles {
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::union(vec![
            SchemaType::boolean(),
            SchemaType::String(StringType {
                enum_values: Some(vec!["args".into(), "env".into()]),
                ..Default::default()
            }),
        ]);
        schema.set_name("TaskOptionAffectedFiles");
        schema
    }
}

impl<'de> Deserialize<'de> for TaskOptionAffectedFiles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::Bool(value) => Ok(TaskOptionAffectedFiles::Enabled(value)),
            Value::String(value) if value == "args" || value == "env" => Ok(if value == "args" {
                TaskOptionAffectedFiles::Args
            } else {
                TaskOptionAffectedFiles::Env
            }),
            _ => Err(de::Error::custom("expected `args`, `env`, or a boolean")),
        }
    }
}

derive_enum!(
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
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::union(vec![
            SchemaType::boolean(),
            SchemaType::string(),
            SchemaType::array(SchemaType::string()),
        ]);
        schema.set_name("TaskOptionEnvFile");
        schema
    }
}

derive_enum!(
    /// The strategy in which to merge a specific task option.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum TaskMergeStrategy {
        #[default]
        Append,
        Prepend,
        Replace,
    }
);

derive_enum!(
    /// The style in which task output will be printed to the console.
    #[derive(ConfigEnum, Copy, Default)]
    pub enum TaskOutputStyle {
        #[default]
        Buffer,
        BufferOnlyFailure,
        Hash,
        None,
        Stream,
    }
);

derive_enum!(
    /// A list of available shells on Unix.
    #[derive(ConfigEnum, Copy)]
    pub enum TaskUnixShell {
        Bash,
        Elvish,
        Fish,
        Zsh,
    }
);

derive_enum!(
    /// A list of available shells on Windows.
    #[derive(ConfigEnum, Copy)]
    pub enum TaskWindowsShell {
        Bash,
        #[serde(alias = "powershell")]
        Pwsh,
    }
);

cacheable!(
    /// Options to control task inheritance and execution.
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskOptionsConfig {
        /// The pattern in which affected files will be passed to the task.
        pub affected_files: Option<TaskOptionAffectedFiles>,

        /// When affected and no files are matching, pass the task inputs
        /// as arguments to the command, instead of `.`.
        pub affected_pass_inputs: Option<bool>,

        /// Allows the task to fail without failing the entire pipeline.
        pub allow_failure: Option<bool>,

        /// Caches the `outputs` of the task
        pub cache: Option<bool>,

        /// Loads and sets environment variables from the `.env` file when
        /// running the task.
        pub env_file: Option<TaskOptionEnvFile>,

        /// Marks the task as interactive, so that it will run in isolation,
        /// and have direct access to stdin.
        #[setting(validate = validate_interactive)]
        pub interactive: Option<bool>,

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

        /// The style in which task output will be printed to the console.
        #[setting(env = "MOON_OUTPUT_STYLE")]
        pub output_style: Option<TaskOutputStyle>,

        /// Marks the task as persistent (continuously running). This is ideal
        /// for watchers, servers, or never-ending processes.
        pub persistent: Option<bool>,

        /// The number of times a failing task will be retried to succeed.
        #[setting(env = "MOON_RETRY_COUNT")]
        pub retry_count: Option<u8>,

        /// Runs direct task dependencies (via `deps`) in sequential order.
        /// This _does not_ apply to indirect or transient dependencies.
        pub run_deps_in_parallel: Option<bool>,

        /// Whether to run the task in CI or not, when executing `moon ci`.
        #[serde(rename = "runInCI")]
        pub run_in_ci: Option<bool>,

        /// Runs the task from the workspace root, instead of the project root.
        pub run_from_workspace_root: Option<bool>,

        /// Runs the task within a shell. When not defined, runs the task
        /// directly while relying on `PATH` resolution.
        pub shell: Option<bool>,

        /// The shell to run the task in when on a Unix-based machine.
        pub unix_shell: Option<TaskUnixShell>,

        /// The shell to run the task in when on a Windows machine.
        pub windows_shell: Option<TaskWindowsShell>,
    }
);
