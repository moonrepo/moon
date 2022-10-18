// .moon/project.yml

use crate::errors::{
    create_validation_error, map_validation_errors_to_figment_errors, ConfigError,
};
use crate::helpers::gather_extended_sources;
use crate::project::task::TaskConfig;
use crate::providers::url::Url;
use crate::types::FileGroups;
use crate::validators::{validate_extends, validate_id};
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

/// Docs: https://moonrepo.dev/docs/config/global-project
#[derive(Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
// We use serde(default) because extended configs may not have defined these fields
#[serde(default, rename_all = "camelCase")]
pub struct GlobalProjectConfig {
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    #[validate(custom = "validate_tasks")]
    #[validate]
    pub tasks: BTreeMap<String, TaskConfig>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl GlobalProjectConfig {
    pub fn load(path: PathBuf) -> Result<GlobalProjectConfig, ConfigError> {
        let profile_name = "globalProject";
        let mut config = GlobalProjectConfig::default();

        for source in gather_extended_sources(&path)? {
            let figment = if source.starts_with("http") {
                Figment::from(Url::from(source).profile(&profile_name))
            } else {
                Figment::from(YamlExtended::file(source).profile(&profile_name))
            };

            let extended_config = GlobalProjectConfig::load_config(figment.select(&profile_name))?;

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

    fn load_config(figment: Figment) -> Result<GlobalProjectConfig, ConfigError> {
        let config: GlobalProjectConfig = figment.extract()?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
