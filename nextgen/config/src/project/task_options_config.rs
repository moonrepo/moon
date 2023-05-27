use crate::portable_path::is_glob;
use moon_common::cacheable;
use schematic::{derive_enum, Config, ConfigEnum, ValidateError};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_yaml::Value;

fn validate_env_file<D, C>(
    env_file: &TaskOptionEnvFile,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    if let TaskOptionEnvFile::File(file) = env_file {
        if is_glob(file) {
            return Err(ValidateError::new("globs are not supported"));
        }
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

impl schemars::JsonSchema for TaskOptionAffectedFiles {
    fn schema_name() -> String {
        "TaskOptionAffectedFiles".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        schemars::schema::Schema::Object(schemars::schema::SchemaObject {
            subschemas: Some(Box::new(schemars::schema::SubschemaValidation {
                one_of: Some(vec![
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::String.into()),
                        enum_values: Some(vec!["args".into(), "env".into()]),
                        ..Default::default()
                    }),
                    schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                        instance_type: Some(schemars::schema::InstanceType::Boolean.into()),
                        ..Default::default()
                    }),
                ]),
                ..Default::default()
            })),
            ..Default::default()
        })
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
        File(String),
    }
);

impl TaskOptionEnvFile {
    pub fn to_option(&self) -> Option<String> {
        match self {
            TaskOptionEnvFile::Enabled(true) => Some(".env".to_owned()),
            TaskOptionEnvFile::Enabled(false) => None,
            TaskOptionEnvFile::File(path) => Some(path.to_owned()),
        }
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

        pub cache: Option<bool>,

        #[setting(validate = validate_env_file)]
        pub env_file: Option<TaskOptionEnvFile>,

        pub merge_args: Option<TaskMergeStrategy>,

        pub merge_deps: Option<TaskMergeStrategy>,

        pub merge_env: Option<TaskMergeStrategy>,

        pub merge_inputs: Option<TaskMergeStrategy>,

        pub merge_outputs: Option<TaskMergeStrategy>,

        pub output_style: Option<TaskOutputStyle>,

        pub persistent: Option<bool>,

        pub retry_count: Option<u8>,

        pub run_deps_in_parallel: Option<bool>,

        #[serde(rename = "runInCI")]
        pub run_in_ci: Option<bool>,

        pub run_from_workspace_root: Option<bool>,

        pub shell: Option<bool>,
    }
);
