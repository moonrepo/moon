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
) -> Result<(), ValidateError> {
    if *enabled && options.persistent.is_some_and(|v| v) {
        return Err(ValidateError::new(
            "an interactive task cannot be persistent",
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged, rename_all = "kebab-case")]
pub enum TaskOptionAffectedFiles {
    Args,
    Env,
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
    #[serde(untagged, expecting = "expected a boolean or a file system path")]
    pub enum TaskOptionEnvFile {
        Enabled(bool),
        File(FilePath),
    }
);

impl TaskOptionEnvFile {
    pub fn to_input_path(&self) -> Option<InputPath> {
        match self {
            TaskOptionEnvFile::Enabled(true) => Some(InputPath::ProjectFile(".env".into())),
            TaskOptionEnvFile::Enabled(false) => None,
            TaskOptionEnvFile::File(path) => InputPath::from_str(path.as_str()).ok(),
        }
    }
}

impl Schematic for TaskOptionEnvFile {
    fn generate_schema() -> SchemaType {
        let mut schema = SchemaType::union(vec![SchemaType::boolean(), SchemaType::string()]);
        schema.set_name("TaskOptionEnvFile");
        schema
    }
}

derive_enum!(
    #[derive(ConfigEnum, Copy, Default)]
    pub enum TaskMergeStrategy {
        #[default]
        Append,
        Prepend,
        Replace,
    }
);

derive_enum!(
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

cacheable!(
    #[derive(Clone, Config, Debug, Eq, PartialEq)]
    pub struct TaskOptionsConfig {
        pub affected_files: Option<TaskOptionAffectedFiles>,

        pub allow_failure: Option<bool>,

        pub cache: Option<bool>,

        pub env_file: Option<TaskOptionEnvFile>,

        #[setting(validate = validate_interactive)]
        pub interactive: Option<bool>,

        pub merge_args: Option<TaskMergeStrategy>,

        pub merge_deps: Option<TaskMergeStrategy>,

        pub merge_env: Option<TaskMergeStrategy>,

        pub merge_inputs: Option<TaskMergeStrategy>,

        pub merge_outputs: Option<TaskMergeStrategy>,

        #[setting(env = "MOON_OUTPUT_STYLE")]
        pub output_style: Option<TaskOutputStyle>,

        pub persistent: Option<bool>,

        #[setting(env = "MOON_RETRY_COUNT")]
        pub retry_count: Option<u8>,

        pub run_deps_in_parallel: Option<bool>,

        #[serde(rename = "runInCI")]
        pub run_in_ci: Option<bool>,

        pub run_from_workspace_root: Option<bool>,

        pub shell: Option<bool>,
    }
);
