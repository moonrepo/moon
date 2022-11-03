// template file frontmatter

use crate::{errors::map_validation_errors_to_figment_errors, ConfigError};
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Docs: https://moonrepo.dev/docs/config/template#frontmatter
#[derive(Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateFrontmatterConfig {
    pub force: Option<bool>,
    pub to: Option<String>,
    pub skip: Option<bool>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl TemplateFrontmatterConfig {
    #[track_caller]
    pub fn parse<T: AsRef<str>>(content: T) -> Result<TemplateFrontmatterConfig, ConfigError> {
        let mut content = content.as_ref();

        if content.is_empty() {
            content = "{}";
        }

        let profile_name = "frontmatter";
        let figment = Figment::from(
            Serialized::defaults(TemplateFrontmatterConfig::default()).profile(profile_name),
        )
        .merge(YamlExtended::string(content).profile(profile_name))
        .select(profile_name);

        let config: TemplateFrontmatterConfig = figment.extract()?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
