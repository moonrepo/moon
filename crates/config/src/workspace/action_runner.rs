use crate::errors::create_validation_error;
use moon_utils::{string_vec, time};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_cache_lifetime(value: &str) -> Result<(), ValidationError> {
    if let Err(e) = time::parse_duration(value) {
        return Err(create_validation_error(
            "invalid_duration",
            "cache_lifetime",
            format!("Invalid lifetime duration: {}", e),
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct ActionRunnerConfig {
    #[validate(custom = "validate_cache_lifetime")]
    pub cache_lifetime: String,

    pub implicit_inputs: Vec<String>,

    pub inherit_colors_for_piped_tasks: bool,

    pub log_running_command: bool,
}

impl Default for ActionRunnerConfig {
    fn default() -> Self {
        ActionRunnerConfig {
            cache_lifetime: "7 days".to_owned(),
            implicit_inputs: string_vec![
                // When a project changes
                "package.json",
                // When root config changes
                "/.moon/project.yml",
                "/.moon/workspace.yml",
            ],
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
        providers::{Format, Serialized, Yaml},
        Figment,
    };
    use std::path::PathBuf;

    const CONFIG_FILENAME: &str = "action-runner.yml";

    fn load_jailed_config() -> Result<ActionRunnerConfig, figment::Error> {
        let figment = Figment::from(Serialized::defaults(ActionRunnerConfig::default()))
            .merge(Yaml::file(&PathBuf::from(CONFIG_FILENAME)));
        let config: ActionRunnerConfig = figment.extract()?;

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
