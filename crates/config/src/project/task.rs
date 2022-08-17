use crate::project::local::{ProjectConfig, ProjectLanguage};
use crate::project::task_options::TaskOptionsConfig;
use crate::types::{FilePath, InputValue, TargetID};
use crate::validators::{skip_if_default, validate_child_or_root_path, validate_target};
use moon_utils::process::split_args;
use moon_utils::regex::{ENV_VAR, NODE_COMMAND, UNIX_SYSTEM_COMMAND, WINDOWS_SYSTEM_COMMAND};
use schemars::gen::SchemaGenerator;
use schemars::schema::Schema;
use schemars::{schema_for, JsonSchema};
use serde::de::{self, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use strum::Display;
use validator::{Validate, ValidationError};

// These structs utilize optional fields so that we can handle merging effectively,
// as we need a way to skip "undefined" values. So don't use serde defaults here.

fn validate_deps(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_target(&format!("deps[{}]", index), item)?;
    }

    Ok(())
}

fn validate_inputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        if !ENV_VAR.is_match(item) {
            validate_child_or_root_path(&format!("inputs[{}]", index), item)?;
        }
    }

    Ok(())
}

fn validate_outputs(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_child_or_root_path(&format!("outputs[{}]", index), item)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformType {
    #[strum(serialize = "node")]
    Node,

    #[strum(serialize = "system")]
    System,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

// We use serde(default) here because figment *does not* apply defaults
// for structs nested within collections. Primarily hash maps.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(default)]
pub struct TaskConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    #[serde(
        deserialize_with = "deserialize_args",
        skip_serializing_if = "Option::is_none"
    )]
    #[schemars(schema_with = "make_args_schema")]
    pub args: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_deps")]
    pub deps: Option<Vec<TargetID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_inputs")]
    pub inputs: Option<Vec<InputValue>>,

    #[serde(skip_serializing_if = "skip_if_default")]
    pub local: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_outputs")]
    pub outputs: Option<Vec<FilePath>>,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[validate]
    pub options: TaskOptionsConfig,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[serde(rename = "type")]
    pub type_of: PlatformType,
}

impl TaskConfig {
    pub fn detect_platform(project: &ProjectConfig, command: &str) -> PlatformType {
        if NODE_COMMAND.is_match(command) {
            return PlatformType::Node;
        }

        if UNIX_SYSTEM_COMMAND.is_match(command) || WINDOWS_SYSTEM_COMMAND.is_match(command) {
            return PlatformType::System;
        }

        match &project.language {
            ProjectLanguage::JavaScript | ProjectLanguage::TypeScript => PlatformType::Node,
            ProjectLanguage::Bash | ProjectLanguage::Batch => PlatformType::System,
            _ => PlatformType::Unknown,
        }
    }
}

// SERDE

struct DeserializeArgs;

impl<'de> de::Visitor<'de> for DeserializeArgs {
    type Value = Vec<String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of strings or a string")
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::new();

        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }

        Ok(vec)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match split_args(value) {
            Ok(args) => Ok(args),
            Err(error) => Err(E::custom(error)),
        }
    }
}

fn deserialize_args<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(deserializer.deserialize_any(DeserializeArgs)?))
}

// JSON SCHEMA

#[derive(JsonSchema)]
#[serde(untagged)]
enum ArgsField {
    #[allow(dead_code)]
    String(String),
    #[allow(dead_code)]
    Sequence(Vec<String>),
}

fn make_args_schema(_gen: &mut SchemaGenerator) -> Schema {
    let root = schema_for!(ArgsField);

    Schema::Object(root.schema)
}
