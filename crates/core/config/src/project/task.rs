use crate::project::language_platform::PlatformType;
use crate::project::task_options::TaskOptionsConfig;
use crate::types::{FilePath, InputValue, TargetID};
use crate::validators::{is_default, validate_child_or_root_path, validate_id, validate_target};
use moon_utils::process::split_args;
use moon_utils::process::ArgsParseError;
use moon_utils::regex::ENV_VAR;
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

// These structs utilize optional fields so that we can handle merging effectively,
// as we need a way to skip "undefined" values. So don't use serde defaults here.

fn validate_deps(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        let key = format!("deps[{index}]");

        // When no target scope, it's assumed to be a self scope
        if item.contains(':') {
            validate_target(key, item)?;
        } else {
            validate_id(key, item)?;
        }
    }

    Ok(())
}

fn validate_inputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        if !ENV_VAR.is_match(item) {
            validate_child_or_root_path(format!("inputs[{index}]"), item)?;
        }
    }

    Ok(())
}

fn validate_outputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_child_or_root_path(format!("outputs[{index}]"), item)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(untagged, expecting = "expected a string or a sequence of strings")]
pub enum TaskCommandArgs {
    String(String),
    Sequence(Vec<String>),
}

// We use serde(default) here because figment *does not* apply defaults
// for structs nested within collections. Primarily hash maps.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct TaskConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<TaskCommandArgs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<TaskCommandArgs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_deps")]
    pub deps: Option<Vec<TargetID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<FxHashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<InputValue>>,

    #[serde(skip_serializing_if = "is_default")]
    pub local: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<FilePath>>,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub options: TaskOptionsConfig,

    #[serde(skip_serializing_if = "is_default")]
    pub platform: PlatformType,
}

impl TaskConfig {
    pub fn get_command(&self) -> String {
        if let Some(cmd) = &self.command {
            match cmd {
                TaskCommandArgs::String(cmd_string) => {
                    let mut parts = cmd_string.split(' ');

                    if let Some(part) = parts.next() {
                        return part.to_owned();
                    }
                }
                TaskCommandArgs::Sequence(cmd_args) => {
                    if !cmd_args.is_empty() {
                        return cmd_args[0].to_owned();
                    }
                }
            };
        }

        String::new()
    }

    pub fn get_command_and_args(&self) -> Result<(Option<String>, Vec<String>), ArgsParseError> {
        let mut command = None;
        let mut args = vec![];

        if let Some(cmd) = &self.command {
            let mut cmd_list = match cmd {
                TaskCommandArgs::String(cmd_string) => split_args(cmd_string)?,
                TaskCommandArgs::Sequence(cmd_args) => cmd_args.clone(),
            };

            if !cmd_list.is_empty() {
                command = Some(cmd_list.remove(0));
                args.extend(cmd_list.clone());
            }
        }

        match &self.args {
            Some(TaskCommandArgs::String(args_string)) => args.extend(split_args(args_string)?),
            Some(TaskCommandArgs::Sequence(args_list)) => args.extend(args_list.clone()),
            _ => {}
        }

        Ok((command, args))
    }
}
