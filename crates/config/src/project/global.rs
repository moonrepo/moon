// .moon/project.yml

use crate::errors::{create_validation_error, map_validation_errors_to_figment_errors};
use crate::project::task::TaskConfig;
use crate::providers::url::Url;
use crate::types::FileGroups;
use crate::validators::{validate_extends, validate_id};
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use validator::{Validate, ValidationError};

fn validate_file_groups(map: &FileGroups) -> Result<(), ValidationError> {
    for key in map.keys() {
        validate_id(&format!("fileGroups.{}", key), key)?;
    }

    Ok(())
}

fn validate_tasks(map: &BTreeMap<String, TaskConfig>) -> Result<(), ValidationError> {
    for (name, task) in map {
        validate_id(&format!("tasks.{}", name), name)?;

        // Fail for both `None` and empty strings
        let command = task.command.clone().unwrap_or_default();

        if command.is_empty() {
            return Err(create_validation_error(
                "required_command",
                &format!("tasks.{}.command", name),
                String::from("An npm/system command is required"),
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
    pub tasks: BTreeMap<String, TaskConfig>,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl GlobalProjectConfig {
    pub fn load(path: PathBuf) -> Result<GlobalProjectConfig, Vec<FigmentError>> {
        let profile_name = "globalProject";
        let mut config = GlobalProjectConfig::load_config(
            Figment::from(
                Serialized::defaults(GlobalProjectConfig::default()).profile(&profile_name),
            )
            .merge(Yaml::file(&path).profile(&profile_name))
            .select(&profile_name),
        )?;

        // This is janky, but figment does not support any kind of extends mechanism,
        // and figment providers do not have access to the current config dataset,
        // so we need to double-load this config and extract in the correct order!
        if let Some(extends) = &config.extends {
            let extended_config =
                GlobalProjectConfig::load_config(if extends.starts_with("http") {
                    Figment::from(Url::from(extends.to_owned()).profile(&profile_name))
                        .select(&profile_name)
                } else {
                    Figment::from(
                        Yaml::file(path.parent().unwrap().join(extends)).profile(&profile_name),
                    )
                    .select(&profile_name)
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
                let mut map = BTreeMap::new();
                map.extend(extended_config.tasks);
                map.extend(config.tasks);

                config.tasks = map;
            }
        }

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<GlobalProjectConfig, Vec<FigmentError>> {
        let config: GlobalProjectConfig = figment.extract().map_err(|e| vec![e])?;

        if let Err(errors) = config.validate() {
            return Err(map_validation_errors_to_figment_errors(&figment, &errors));
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment;
    use moon_constants as constants;
    use moon_utils::string_vec;
    use std::path::Path;

    fn load_jailed_config(root: &Path) -> Result<GlobalProjectConfig, figment::Error> {
        match GlobalProjectConfig::load(root.join(constants::CONFIG_PROJECT_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(errors.first().unwrap().clone()),
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
                    tasks: BTreeMap::new(),
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"globalProject.extends\""
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
            expected = "Must be a valid URL or relative file path (starts with ./) for key \"globalProject.extends\""
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
        #[should_panic(
            expected = "Only HTTPS URLs are supported for key \"globalProject.extends\""
        )]
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
        #[should_panic(expected = "Must be a YAML document for key \"globalProject.extends\"")]
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
        #[should_panic(expected = "Must be a YAML document for key \"globalProject.extends\"")]
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

        fn create_merged_tasks() -> BTreeMap<String, TaskConfig> {
            BTreeMap::from([
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
            use pretty_assertions::assert_eq;

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
            use pretty_assertions::assert_eq;

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

        //         #[test]
        //         #[should_panic(expected = "TODO")]
        //         fn handles_invalid_url() {
        //             figment::Jail::expect_with(|jail| {
        //                 jail.create_file(
        //                     super::constants::CONFIG_PROJECT_FILENAME,
        //                     r#"
        // extends: https://raw.githubusercontent.com/this/is/an/invalid/file.yml

        // fileGroups: {}
        // "#,
        //                 )?;

        //                 super::load_jailed_config(jail.directory())?;

        //                 Ok(())
        //             });
        //         }
    }

    mod file_groups {
        #[test]
        #[should_panic(
            expected = "invalid type: found unsigned int `123`, expected a map for key \"globalProject.fileGroups\""
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
            expected = "invalid type: found unsigned int `123`, expected a sequence for key \"globalProject.fileGroups.sources\""
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
            expected = "invalid type: found unsigned int `123`, expected a map for key \"globalProject.tasks\""
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
            expected = "invalid type: found unsigned int `123`, expected struct TaskConfig for key \"globalProject.tasks.test\""
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"globalProject.tasks.test.command\""
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
        #[should_panic(expected = "An npm/system command is required")]
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
