use crate::{
    errors::create_validation_error,
    validators::{is_default, is_default_true, validate_target},
};
use moon_utils::time;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_cache_lifetime(value: &str) -> Result<(), ValidationError> {
    if let Err(e) = time::parse_duration(value) {
        return Err(create_validation_error(
            "invalid_duration",
            "cacheLifetime",
            format!("Invalid lifetime duration: {e}"),
        ));
    }

    Ok(())
}

fn validate_archivable_targets(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_target(format!("archivableTargets[{index}]"), item)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct RunnerConfig {
    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_archivable_targets")]
    pub archivable_targets: Vec<String>,

    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_cache_lifetime")]
    pub cache_lifetime: String,

    #[serde(skip_serializing_if = "is_default_true")]
    pub inherit_colors_for_piped_tasks: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub log_running_command: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        RunnerConfig {
            cache_lifetime: "7 days".to_owned(),
            archivable_targets: vec![],
            inherit_colors_for_piped_tasks: true,
            log_running_command: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::map_validation_errors_to_figment_errors;
    use figment::{
        providers::{Format, Serialized, YamlExtended},
        Figment,
    };
    use std::path::PathBuf;

    const CONFIG_FILENAME: &str = "runner.yml";

    fn load_jailed_config() -> Result<RunnerConfig, figment::Error> {
        let figment = Figment::from(Serialized::defaults(RunnerConfig::default()))
            .merge(YamlExtended::file(PathBuf::from(CONFIG_FILENAME)));
        let config: RunnerConfig = figment.extract()?;

        config
            .validate()
            .map_err(|e| map_validation_errors_to_figment_errors(&figment, &e)[0].clone())?;

        Ok(config)
    }

    #[test]
    #[should_panic(expected = "Invalid lifetime duration: expected number at 0")]
    fn invalid_cache_lifetime() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_FILENAME,
                r#"
cacheLifetime: 'bad unit'
"#,
            )?;

            load_jailed_config()?;

            Ok(())
        });
    }
}
