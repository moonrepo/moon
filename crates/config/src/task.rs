use crate::validators::validate_child_or_root_path;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::{Validate, ValidationError};

fn validate_inputs(list: &[String]) -> Result<(), ValidationError> {
    for item in list {
        validate_child_or_root_path(&format!("inputs[{}]", 0), item)?;
    }

    Ok(())
}

fn validate_outputs(list: &[String]) -> Result<(), ValidationError> {
    for item in list {
        validate_child_or_root_path(&format!("outputs[{}]", 0), item)?;
    }

    Ok(())
}

pub type Tasks = HashMap<String, TaskConfig>;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    Npm,
    Shell,
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::Npm
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskMergeStrategy {
    Append,
    Prepend,
    Replace,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct TaskOptionsConfig {
    pub merge_strategy: Option<TaskMergeStrategy>,

    pub retry_count: Option<u8>,
}

impl Default for TaskOptionsConfig {
    fn default() -> Self {
        TaskOptionsConfig {
            merge_strategy: Some(TaskMergeStrategy::Append),
            retry_count: Some(0),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize, Validate)]
pub struct TaskConfig {
    pub args: Option<Vec<String>>,

    pub command: String,

    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<String>>,

    pub options: Option<TaskOptionsConfig>,

    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<String>>,

    #[serde(rename = "type")]
    pub type_of: Option<TaskType>,
}
