// template.yml

use crate::helpers::warn_for_unknown_fields;
use crate::validators::{is_default, validate_non_empty};
use crate::{errors::map_validation_errors_to_figment_errors, ConfigError};
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use validator::{Validate, ValidationError};

fn validate_description(value: &str) -> Result<(), ValidationError> {
    validate_non_empty("description", value)?;

    Ok(())
}

fn validate_title(value: &str) -> Result<(), ValidationError> {
    validate_non_empty("title", value)?;

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
pub struct TemplateVariableConfig<T> {
    pub default: T,
    pub prompt: Option<String>,
    pub required: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    untagged,
    expecting = "expected a value string or value object with label"
)]
pub enum TemplateVariableEnumValue {
    String(String),
    Object { label: String, value: String },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
pub struct TemplateVariableEnumConfig {
    pub default: String,
    pub multiple: Option<bool>,
    pub prompt: String,
    // pub required: Option<bool>,
    pub values: Vec<TemplateVariableEnumValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum TemplateVariable {
    Boolean(TemplateVariableConfig<bool>),
    Enum(TemplateVariableEnumConfig),
    Number(TemplateVariableConfig<i32>),
    // NumberList(TemplateVariableConfig<Vec<i32>>),
    String(TemplateVariableConfig<String>),
    // StringList(TemplateVariableConfig<Vec<String>>),
}

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct TemplateConfig {
    #[validate(custom = "validate_description")]
    pub description: String,

    #[validate(custom = "validate_title")]
    pub title: String,

    #[schemars(default)]
    pub variables: FxHashMap<String, TemplateVariable>,

    /// JSON schema URI
    #[serde(rename = "$schema", skip_serializing_if = "is_default")]
    pub schema: String,

    /// Unknown fields
    #[serde(flatten)]
    #[schemars(skip)]
    pub unknown: BTreeMap<String, serde_yaml::Value>,
}

impl TemplateConfig {
    #[track_caller]
    pub fn load<T: AsRef<Path>>(path: T) -> Result<TemplateConfig, ConfigError> {
        let path = path.as_ref();
        let profile_name = "template";
        let figment =
            Figment::from(Serialized::defaults(TemplateConfig::default()).profile(profile_name))
                .merge(YamlExtended::file(path).profile(profile_name))
                .select(profile_name);

        let config: TemplateConfig = figment.extract()?;

        warn_for_unknown_fields(&path, &config.unknown);

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
