use crate::validators::validate_child_relative_path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_env_file(file: &TaskOptionEnvFile) -> Result<(), ValidationError> {
    if let TaskOptionEnvFile::File(path) = file {
        validate_child_relative_path("envFile", path)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    untagged,
    expecting = "expected a boolean or a relative file system path"
)]
pub enum TaskOptionEnvFile {
    Enabled(bool),
    File(String),
}

impl TaskOptionEnvFile {
    pub fn to_option(&self) -> Option<String> {
        match self {
            TaskOptionEnvFile::Enabled(true) => Some(".env".to_owned()),
            TaskOptionEnvFile::Enabled(false) => None,
            TaskOptionEnvFile::File(path) => Some(path.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskMergeStrategy {
    #[default]
    Append,
    Prepend,
    Replace,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskOutputStyle {
    Buffer,
    BufferOnlyFailure,
    Hash,
    None,
    Stream,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskOptionsConfig {
    pub cache: Option<bool>,

    #[validate(custom = "validate_env_file")]
    pub env_file: Option<TaskOptionEnvFile>,

    pub merge_args: Option<TaskMergeStrategy>,

    pub merge_deps: Option<TaskMergeStrategy>,

    pub merge_env: Option<TaskMergeStrategy>,

    pub merge_inputs: Option<TaskMergeStrategy>,

    pub merge_outputs: Option<TaskMergeStrategy>,

    pub output_style: Option<TaskOutputStyle>,

    pub retry_count: Option<u8>,

    pub run_deps_in_parallel: Option<bool>,

    #[serde(rename = "runInCI")]
    pub run_in_ci: Option<bool>,

    pub run_from_workspace_root: Option<bool>,
}
