// .moon/tasks.yml

use crate::errors::{
    create_validation_error, map_validation_errors_to_figment_errors, ConfigError,
};
use crate::helpers::gather_extended_sources;
use crate::project::TaskConfig;
use crate::types::FileGroups;
use crate::validators::{is_default, validate_extends, validate_id};
use figment::{
    providers::{Format, YamlExtended},
    Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use validator::{Validate, ValidationError};

fn validate_file_groups(map: &FileGroups) -> Result<(), ValidationError> {
    for key in map.keys() {
        validate_id(format!("fileGroups.{}", key), key)?;
    }

    Ok(())
}

fn validate_tasks(map: &BTreeMap<String, TaskConfig>) -> Result<(), ValidationError> {
    for (name, task) in map {
        validate_id(format!("tasks.{}", name), name)?;

        // Fail for both `None` and empty strings
        if task.get_command().is_empty() {
            return Err(create_validation_error(
                "required_command",
                format!("tasks.{}.command", name),
                "An npm/system command is required",
            ));
        }
    }

    Ok(())
}

/// Docs: https://moonrepo.dev/docs/config/tasks
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct InheritedTasksConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_tasks")]
    #[validate]
    pub tasks: BTreeMap<String, TaskConfig>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl InheritedTasksConfig {
    pub fn load(path: PathBuf) -> Result<InheritedTasksConfig, ConfigError> {
        let profile_name = "inheritedTasks";
        let mut config = InheritedTasksConfig::default();

        for source in gather_extended_sources(path)? {
            let figment = Figment::from(YamlExtended::file(source).profile(profile_name));
            let extended_config = InheritedTasksConfig::load_config(figment.select(profile_name))?;

            // Figment does not merge hash maps but replaces entirely,
            // so we need to manually handle this here!
            if !extended_config.file_groups.is_empty() {
                config.file_groups.extend(extended_config.file_groups);
            }

            if !extended_config.tasks.is_empty() {
                config.tasks.extend(extended_config.tasks);
            }
        }

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<InheritedTasksConfig, ConfigError> {
        let config: InheritedTasksConfig = figment.extract()?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
