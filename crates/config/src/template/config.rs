// template.yml

use crate::validators::validate_non_empty;
use crate::{errors::map_validation_errors_to_figment_errors, ConfigError};
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateConfig {
    #[validate(custom = "validate_description")]
    pub description: String,

    #[validate(custom = "validate_title")]
    pub title: String,
}

impl TemplateConfig {
    #[track_caller]
    pub fn load<T: AsRef<Path>>(path: T) -> Result<TemplateConfig, ConfigError> {
        let path = path.as_ref();
        let profile_name = "template";
        let figment =
            Figment::from(Serialized::defaults(TemplateConfig::default()).profile(&profile_name))
                .merge(Yaml::file(path).profile(&profile_name))
                .select(&profile_name);

        let config: TemplateConfig = figment.extract()?; //.map_err(|e| vec![e])?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
