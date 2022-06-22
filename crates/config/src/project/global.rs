// .moon/project.yml

use crate::constants;
use crate::errors::{create_validation_error, map_figment_error_to_validation_errors};
use crate::project::task::TaskConfig;
use crate::providers::url::Url;
use crate::types::FileGroups;
use crate::validators::{validate_extends, validate_id};
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
                String::from("An npm/system command is required."),
            ));
        }
    }

    Ok(())
}

/// Docs: https://moonrepo.dev/docs/config/global-project
#[derive(Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProjectConfig {
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[schemars(default)]
    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    #[schemars(default)]
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
        Serialized::defaults(self).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl GlobalProjectConfig {
    pub fn load(path: PathBuf) -> Result<GlobalProjectConfig, ValidationErrors> {
        let mut config = GlobalProjectConfig::load_config(
            Figment::from(GlobalProjectConfig::default()).merge(Yaml::file(&path)),
        )?;

        // This is janky, but figment does not support any kind of extends mechanism,
        // and figment providers do not have access to the current config dataset,
        // so we need to double-load this config and extract in the correct order!
        if let Some(extends) = &config.extends {
            let extended_config =
                GlobalProjectConfig::load_config(if extends.starts_with("http") {
                    Figment::from(Url::from(extends.to_owned()))
                } else {
                    Figment::from(Yaml::file(path.parent().unwrap().join(extends)))
                })?;

            // Figment does not merge hash maps but replaces entirely,
            // so we need to manually handle this here!
            if !extended_config.file_groups.is_empty() {
                let mut map = HashMap::new();
                map.extend(extended_config.file_groups);
                map.extend(config.file_groups);

                config.file_groups = map;
            }

            if !extended_config.tasks.is_empty() {
                let mut map = HashMap::new();
                map.extend(extended_config.tasks);
                map.extend(config.tasks);

                config.tasks = map;
            }
        }

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<GlobalProjectConfig, ValidationErrors> {
        let config: GlobalProjectConfig = match figment.extract() {
            Ok(cfg) => cfg,
            Err(error) => return Err(map_figment_error_to_validation_errors(&error)),
        };

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
    use std::path::Path;

    fn load_jailed_config(root: &Path) -> Result<GlobalProjectConfig, figment::Error> {
        match GlobalProjectConfig::load(root.join(constants::CONFIG_PROJECT_FILENAME)) {
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

            let config = load_jailed_config(jail.directory())?;

            assert_eq!(
                config,
                GlobalProjectConfig {
                    extends: None,
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

    mod extends {
        use super::*;
        use crate::project::task::TaskOptionsConfig;
        use std::fs;

        #[test]
        #[should_panic(
            expected = "Invalid field <id>extends</id>: Expected a string type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "extends: 123")?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>extends</id>: Must be a valid URL or relative file path (starts with ./)."
        )]
        fn not_a_url_or_file() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    "extends: random value",
                )?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(expected = "Invalid field <id>extends</id>: Only HTTPS URLs are supported.")]
        fn not_a_https_url() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    "extends: http://domain.com",
                )?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(expected = "Invalid field <id>extends</id>: Must be a YAML document.")]
        fn not_a_yaml_url() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    "extends: https://domain.com/file.txt",
                )?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(expected = "Invalid field <id>extends</id>: Must be a YAML document.")]
        fn not_a_yaml_file() {
            figment::Jail::expect_with(|jail| {
                fs::create_dir_all(jail.directory().join("shared")).unwrap();

                jail.create_file("shared/file.txt", "")?;

                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    "extends: ./shared/file.txt",
                )?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        fn create_merged_tasks() -> HashMap<String, TaskConfig> {
            HashMap::from([
                (
                    "onlyCommand".to_owned(),
                    TaskConfig {
                        command: Some(String::from("a")),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "stringArgs".to_owned(),
                    TaskConfig {
                        command: Some(String::from("b")),
                        args: Some(string_vec!["string", "args"]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "arrayArgs".to_owned(),
                    TaskConfig {
                        command: Some(String::from("c")),
                        args: Some(string_vec!["array", "args"]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "inputs".to_owned(),
                    TaskConfig {
                        command: Some(String::from("d")),
                        inputs: Some(string_vec!["src/**/*"]),
                        ..TaskConfig::default()
                    },
                ),
                (
                    "options".to_owned(),
                    TaskConfig {
                        command: Some(String::from("e")),
                        options: TaskOptionsConfig {
                            merge_args: None,
                            merge_deps: None,
                            merge_env: None,
                            merge_inputs: None,
                            merge_outputs: None,
                            retry_count: None,
                            run_in_ci: Some(false),
                            run_from_workspace_root: None,
                        },
                        ..TaskConfig::default()
                    },
                ),
            ])
        }

        #[test]
        fn loads_from_file() {
            figment::Jail::expect_with(|jail| {
                fs::create_dir_all(jail.directory().join("shared")).unwrap();

                jail.create_file(
                    format!("shared/{}", super::constants::CONFIG_PROJECT_FILENAME),
                    include_str!("../../../../tests/fixtures/config-extends/.moon/project.yml"),
                )?;

                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
extends: ./shared/project.yml

fileGroups:
    sources:
        - sources/**/*
    configs:
        - '*.js'
"#,
                )?;

                let config: GlobalProjectConfig = super::load_jailed_config(jail.directory())?;

                assert_eq!(config.extends, Some("./shared/project.yml".to_owned()));

                // Ensure values are deep merged
                assert_eq!(
                    config.file_groups,
                    HashMap::from([
                        ("sources".to_owned(), string_vec!["sources/**/*"]), // NOT src/**/*
                        ("tests".to_owned(), string_vec!["tests/**/*"]),
                        ("configs".to_owned(), string_vec!["*.js"])
                    ])
                );

                assert_eq!(config.tasks, create_merged_tasks());

                Ok(())
            });
        }

        #[test]
        fn loads_from_url() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
r#"
extends: https://raw.githubusercontent.com/moonrepo/moon/master/tests/fixtures/config-extends/.moon/project.yml

fileGroups:
    sources:
        - sources/**/*
    configs:
        - '*.js'
"#,
                )?;

                let config: GlobalProjectConfig = super::load_jailed_config(jail.directory())?;

                assert_eq!(
                    config.extends,
                    Some("https://raw.githubusercontent.com/moonrepo/moon/master/tests/fixtures/config-extends/.moon/project.yml".to_owned())
                );

                // Ensure values are deep merged
                assert_eq!(
                    config.file_groups,
                    HashMap::from([
                        ("sources".to_owned(), string_vec!["sources/**/*"]), // NOT src/**/*
                        ("tests".to_owned(), string_vec!["tests/**/*"]),
                        ("configs".to_owned(), string_vec!["*.js"])
                    ])
                );

                assert_eq!(config.tasks, create_merged_tasks());

                Ok(())
            });
        }
    }

    mod file_groups {
        #[test]
        #[should_panic(
            expected = "Invalid field <id>fileGroups</id>: Expected a map type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "fileGroups: 123")?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>fileGroups.sources</id>: Expected a sequence type, received unsigned int `123`."
        )]
        fn invalid_value_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
fileGroups:
    sources: 123"#,
                )?;

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }
    }

    mod tasks {
        #[test]
        #[should_panic(
            expected = "Invalid field <id>tasks</id>: Expected a map type, received unsigned int `123`."
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

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>tasks.test</id>: Expected struct TaskConfig type, received unsigned int `123`."
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

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>tasks.test.command</id>: Expected a string type, received unsigned int `123`."
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

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>tasks.test.command</id>: An npm/system command is required."
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

                super::load_jailed_config(jail.directory())?;

                Ok(())
            });
        }
    }
}
