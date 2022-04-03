// .moon/project.yml
#![allow(rustdoc::bare_urls)]

use crate::constants;
use crate::errors::{create_validation_error, map_figment_error_to_validation_errors};
use crate::project::task::TaskConfig;
use crate::types::FileGroups;
use crate::validators::{validate_id, HashMapValidate};
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment, Metadata, Profile, Provider,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::{Validate, ValidationError, ValidationErrors};

fn validate_file_groups(map: &FileGroups) -> Result<(), ValidationError> {
    for key in map.keys() {
        validate_id(&format!("fileGroups.{}", key), key)?;
    }

    Ok(())
}

fn validate_tasks(map: &HashMap<String, TaskConfig>) -> Result<(), ValidationError> {
    for (name, task) in map {
        validate_id(&format!("tasks.{}", name), name)?;

        // Fail for both `None` and empty strings
        let command = task.command.clone().unwrap_or_default();

        if command.is_empty() {
            return Err(create_validation_error(
                "required_command",
                &format!("tasks.{}.command", name),
                String::from("An npm/shell command is required."),
            ));
        }
    }

    Ok(())
}

/// https://moonrepo.dev/docs/config/global-project
#[derive(Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProjectConfig {
    #[serde(default)]
    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    #[serde(default)]
    #[validate(custom = "validate_tasks")]
    #[validate]
    pub tasks: HashMap<String, TaskConfig>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl Provider for GlobalProjectConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("Global project config").source(format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_PROJECT_FILENAME
        ))
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        Serialized::defaults(GlobalProjectConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl GlobalProjectConfig {
    pub fn load(path: PathBuf) -> Result<GlobalProjectConfig, ValidationErrors> {
        let config: GlobalProjectConfig =
            match Figment::from(Serialized::defaults(GlobalProjectConfig::default()))
                .merge(Yaml::file(path))
                .extract()
            {
                Ok(cfg) => cfg,
                Err(error) => return Err(map_figment_error_to_validation_errors(&error)),
            };

        // Validate the fields before continuing
        if let Err(errors) = config.validate() {
            return Err(errors);
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::tests::handled_jailed_error;
    use figment;
    use moon_utils::string_vec;

    fn load_jailed_config() -> Result<GlobalProjectConfig, figment::Error> {
        match GlobalProjectConfig::load(PathBuf::from(constants::CONFIG_PROJECT_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(handled_jailed_error(&errors)),
        }
    }

    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                constants::CONFIG_PROJECT_FILENAME,
                r#"
fileGroups:
    sources:
        - src/**/*"#,
            )?;

            let config = load_jailed_config()?;

            assert_eq!(
                config,
                GlobalProjectConfig {
                    file_groups: HashMap::from([(
                        String::from("sources"),
                        string_vec!["src/**/*"]
                    )]),
                    tasks: HashMap::new(),
                    schema: String::new(),
                }
            );

            Ok(())
        });
    }

    mod file_groups {
        #[test]
        #[should_panic(
            expected = "Invalid field `fileGroups`. Expected a map type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "fileGroups: 123")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `fileGroups.sources`. Expected a sequence type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups:
    sources: 123"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod tasks {
        #[test]
        #[should_panic(
            expected = "Invalid field `tasks`. Expected a map type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups: {}
tasks: 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `tasks.test`. Expected struct TaskConfig type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups: {}
tasks:
    test: 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `tasks.test.command`. Expected a string type, received unsigned int `123`."
        )]
        fn invalid_value_field() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups: {}
tasks:
    test:
        command: 123
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `tasks.test.command`. An npm/shell command is required."
        )]
        fn invalid_value_empty_field() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups:
    sources: []
tasks:
    test: {}
"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }
}
