// template.yml

use crate::errors::map_validation_errors_to_figment_errors;
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use validator::Validate;

/// Docs: https://moonrepo.dev/docs/config/template
#[derive(Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateConfig {
    #[validate(length(min = 1))]
    pub description: String,

    #[validate(length(min = 1))]
    pub title: String,
}

impl TemplateConfig {
    #[track_caller]
    pub fn load<T: AsRef<Path>>(path: T) -> Result<TemplateConfig, Vec<FigmentError>> {
        let path = path.as_ref();
        let profile_name = "template";
        let figment =
            Figment::from(Serialized::defaults(TemplateConfig::default()).profile(&profile_name))
                .merge(Yaml::file(path).profile(&profile_name))
                .select(&profile_name);

        let config: TemplateConfig = figment.extract().map_err(|e| vec![e])?;

        if let Err(errors) = config.validate() {
            return Err(map_validation_errors_to_figment_errors(&figment, &errors));
        }

        Ok(config)
    }
}
