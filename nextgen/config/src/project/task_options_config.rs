use crate::portable_path::PortablePath;
use schematic::{config_enum, Config, ValidateError};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_yaml::Value;

fn validate_env_file<D, C>(
    env_file: &TaskOptionEnvFile,
    _data: &D,
    _ctx: &C,
) -> Result<(), ValidateError> {
    if let TaskOptionEnvFile::File(file) = env_file {
        match file {
            PortablePath::EnvVar(_) => {
                return Err(ValidateError::new(
                    "environment variables are not supported",
                ));
            }
            PortablePath::ProjectGlob(_) | PortablePath::WorkspaceGlob(_) => {
                return Err(ValidateError::new("globs are not supported"));
            }
            _ => {}
        };
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

config_enum!(
    #[serde(untagged, expecting = "expected a boolean or a file system path")]
    pub enum TaskOptionEnvFile {
        Enabled(bool),
        File(PortablePath),
    }
);

impl TaskOptionEnvFile {
    pub fn to_option(&self) -> Option<String> {
        match self {
            TaskOptionEnvFile::Enabled(true) => Some(".env".to_owned()),
            TaskOptionEnvFile::Enabled(false) => None,
            TaskOptionEnvFile::File(_path) => Some("".into()), // TODO
        }
    }
}

config_enum!(
    #[derive(Default)]
    pub enum TaskMergeStrategy {
        #[default]
        Append,
        Prepend,
        Replace,
    }
);

config_enum!(
    #[derive(Default)]
    pub enum TaskOutputStyle {
        #[default]
        Buffer,
        BufferOnlyFailure,
        Hash,
        None,
        Stream,
    }
);

#[derive(Debug, Clone, Config, Deserialize, Serialize)]
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

    #[setting(rename = "runInCI")]
    pub run_in_ci: Option<bool>,

    pub run_from_workspace_root: Option<bool>,

    pub shell: Option<bool>,
}
