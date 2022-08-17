use figment;
use moon_config::{ConfigError, GlobalProjectConfig};
use moon_constants::CONFIG_GLOBAL_PROJECT_FILENAME;
use moon_utils::string_vec;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

fn load_jailed_config(root: &Path) -> Result<GlobalProjectConfig, figment::Error> {
    match GlobalProjectConfig::load(root.join(CONFIG_GLOBAL_PROJECT_FILENAME)) {
        Ok(cfg) => Ok(cfg),
        Err(error) => Err(match error {
            ConfigError::FailedValidation(errors) => errors.first().unwrap().to_owned(),
            ConfigError::Figment(f) => f,
            e => figment::Error::from(e.to_string()),
        }),
    }
}

#[test]
fn loads_defaults() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            CONFIG_GLOBAL_PROJECT_FILENAME,
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
                file_groups: HashMap::from([(String::from("sources"), string_vec!["src/**/*"])]),
                tasks: BTreeMap::new(),
                schema: String::new(),
            }
        );

        Ok(())
    });
}

mod extends {
    use super::*;
    use moon_config::{TaskConfig, TaskOptionsConfig};
    use std::fs;

    #[test]
    // #[should_panic(
    //     expected = "invalid type: found unsigned int `123`, expected a string for key \"globalProject.extends\""
    // )]
    #[should_panic(expected = "Invalid <id>extends</id> field, must be a string.")]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_GLOBAL_PROJECT_FILENAME, "extends: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(
    //     expected = "Must be a valid URL or relative file path (starts with ./) for key \"globalProject.extends\""
    // )]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_url_or_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
                "extends: random value",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only HTTPS URLs are supported")]
    fn not_a_https_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
                "extends: http://domain.com/config.yml",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(expected = "Must be a YAML document for key \"globalProject.extends\"")]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_yaml_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
                "extends: https://domain.com/file.txt",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(expected = "Must be a YAML document for key \"globalProject.extends\"")]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_yaml_file() {
        figment::Jail::expect_with(|jail| {
            fs::create_dir_all(jail.directory().join("shared")).unwrap();

            jail.create_file("shared/file.txt", "")?;

            jail.create_file(
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                        run_in_ci: Some(false),
                        ..TaskOptionsConfig::default()
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
                format!("shared/{}", super::CONFIG_GLOBAL_PROJECT_FILENAME),
                include_str!("../../../tests/fixtures/config-extends/.moon/project.yml"),
            )?;

            jail.create_file(
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                    super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
    //                     super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
            jail.create_file(super::CONFIG_GLOBAL_PROJECT_FILENAME, "fileGroups: 123")?;

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
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
                super::CONFIG_GLOBAL_PROJECT_FILENAME,
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
