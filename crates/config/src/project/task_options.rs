use crate::validators::validate_child_relative_path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_env_file(file: &TaskOptionEnvFile) -> Result<(), ValidationError> {
    if let TaskOptionEnvFile::File(path) = file {
        validate_child_relative_path("env_file", path)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(untagged)]
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
    BufferOnFailure,
    Hash,
    None,
    Stream,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(default, rename_all = "camelCase")]
pub struct TaskOptionsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_env_file")]
    pub env_file: Option<TaskOptionEnvFile>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_args: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_deps: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_env: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_inputs: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_outputs: Option<TaskMergeStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_style: Option<TaskOutputStyle>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_deps_in_parallel: Option<bool>,

    #[serde(rename = "runInCI", skip_serializing_if = "Option::is_none")]
    pub run_in_ci: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_from_workspace_root: Option<bool>,
}
