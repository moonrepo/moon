use crate::{
    errors::create_validation_error,
    validators::{is_default, validate_id, validate_target},
};
use moon_utils::{string_vec, time};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

fn validate_deps(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        let key = format!("implicitDeps[{}]", index);

        // When no target scope, it's assumed to be a self scope
        if item.contains(':') {
            validate_target(key, item)?;
        } else {
            validate_id(key, item)?;
        }
    }

    Ok(())
}

fn validate_cache_lifetime(value: &str) -> Result<(), ValidationError> {
    if let Err(e) = time::parse_duration(value) {
        return Err(create_validation_error(
            "invalid_duration",
            "cacheLifetime",
            format!("Invalid lifetime duration: {}", e),
        ));
    }

    Ok(())
}

fn validate_archivable_targets(list: &[String]) -> Result<(), ValidationError> {
    for (index, item) in list.iter().enumerate() {
        validate_target(format!("archivableTargets[{}]", index), item)?;
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

    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_deps")]
    pub implicit_deps: Vec<String>,

    #[serde(skip_serializing_if = "is_default")]
    pub implicit_inputs: Vec<String>,

    #[serde(skip_serializing_if = "is_default")]
    pub inherit_colors_for_piped_tasks: bool,

    #[serde(skip_serializing_if = "is_default")]
    pub log_running_command: bool,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        RunnerConfig {
            cache_lifetime: "7 days".to_owned(),
            archivable_targets: vec![],
            implicit_deps: vec![],
            implicit_inputs: string_vec![
                // When a project changes
                "package.json",
                // When root config changes
                "/.moon/project.yml",
                "/.moon/toolchain.yml",
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

    #[test]
    #[should_panic(expected = "Must be a valid target format")]
    fn invalid_dep_target() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_FILENAME,
                r#"
implicitDeps:
  - '%:task'
"#,
            )?;

            load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "Must be a valid ID (accepts A-Z, a-z, 0-9, - (dashes), _ (underscores), /, and must start with a letter)"
    )]
    fn invalid_dep_target_no_scope() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_FILENAME,
                r#"
implicitDeps:
  - 'foo bar'
"#,
            )?;

            load_jailed_config()?;

            Ok(())
        });
    }
}
