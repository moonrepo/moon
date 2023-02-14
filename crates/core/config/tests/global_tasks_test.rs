use httpmock::prelude::*;
use moon_config::{ConfigError, InheritedTasksConfig, TaskCommandArgs};
use moon_constants::CONFIG_TASKS_FILENAME;
use moon_test_utils::get_fixtures_path;
use moon_utils::string_vec;
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::path::Path;

fn load_jailed_config(root: &Path) -> Result<InheritedTasksConfig, figment::Error> {
    match InheritedTasksConfig::load(root.join(CONFIG_TASKS_FILENAME)) {
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
            CONFIG_TASKS_FILENAME,
            r#"
fileGroups:
    sources:
        - src/**/*"#,
        )?;

        let config = load_jailed_config(jail.directory())?;

        assert_eq!(
            config,
            InheritedTasksConfig {
                file_groups: FxHashMap::from_iter([(
                    String::from("sources"),
                    string_vec!["src/**/*"]
                )]),
                ..InheritedTasksConfig::default()
            }
        );

        Ok(())
    });
}

#[test]
#[should_panic(expected = "Must be a valid target format")]
fn invalid_dep_target() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            CONFIG_TASKS_FILENAME,
            r#"
implicitDeps:
  - '%:task'
"#,
        )?;

        load_jailed_config(jail.directory())?;

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
            CONFIG_TASKS_FILENAME,
            r#"
implicitDeps:
  - 'foo bar'
"#,
        )?;

        load_jailed_config(jail.directory())?;

        Ok(())
    });
}

mod extends {
    use super::*;
    use moon_config::{TaskConfig, TaskOptionsConfig};
    use moon_test_utils::pretty_assertions::assert_eq;
    use std::fs;

    #[test]
    fn recursive_merges() {
        let fixture = get_fixtures_path("config-extends/project");
        let config = InheritedTasksConfig::load(fixture.join("global-2.yml")).unwrap();

        assert_eq!(
            config,
            InheritedTasksConfig {
                file_groups: FxHashMap::from_iter([
                    ("tests".to_owned(), string_vec!["tests/**/*"]),
                    ("sources".to_owned(), string_vec!["sources/**/*"]), // NOT src/**/*
                ]),
                tasks: BTreeMap::from([
                    (
                        "lint".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("eslint".to_owned())),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        "format".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("prettier".to_owned())),
                            ..TaskConfig::default()
                        },
                    ),
                    (
                        "test".to_owned(),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("noop".to_owned())),
                            ..TaskConfig::default()
                        },
                    )
                ]),
                ..InheritedTasksConfig::default()
            }
        )
    }

    #[test]
    // #[should_panic(
    //     expected = "invalid type: found unsigned int `123`, expected a string for key \"inheritedTasks.extends\""
    // )]
    #[should_panic(expected = "Invalid <id>extends</id> field, must be a string.")]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TASKS_FILENAME, "extends: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(
    //     expected = "Must be a valid URL or relative file path (starts with ./) for key \"inheritedTasks.extends\""
    // )]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_url_or_file() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TASKS_FILENAME, "extends: random value")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "only HTTPS URLs are supported")]
    fn not_a_https_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                "extends: http://domain.com/config.yml",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(expected = "Must be a YAML document for key \"inheritedTasks.extends\"")]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_yaml_url() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                "extends: https://domain.com/file.txt",
            )?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    // #[should_panic(expected = "Must be a YAML document for key \"inheritedTasks.extends\"")]
    #[should_panic(expected = "only YAML documents are supported")]
    fn not_a_yaml_file() {
        figment::Jail::expect_with(|jail| {
            fs::create_dir_all(jail.directory().join("shared")).unwrap();

            jail.create_file("shared/file.txt", "")?;

            jail.create_file(super::CONFIG_TASKS_FILENAME, "extends: ./shared/file.txt")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    fn create_merged_tasks() -> BTreeMap<String, TaskConfig> {
        BTreeMap::from([
            (
                "onlyCommand".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("a".to_owned())),
                    ..TaskConfig::default()
                },
            ),
            (
                "stringArgs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("b".to_owned())),
                    args: Some(TaskCommandArgs::String("string args".to_owned())),
                    ..TaskConfig::default()
                },
            ),
            (
                "arrayArgs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("c".to_owned())),
                    args: Some(TaskCommandArgs::Sequence(string_vec!["array", "args"])),
                    ..TaskConfig::default()
                },
            ),
            (
                "inputs".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("d".to_owned())),
                    inputs: Some(string_vec!["src/**/*"]),
                    ..TaskConfig::default()
                },
            ),
            (
                "options".to_owned(),
                TaskConfig {
                    command: Some(TaskCommandArgs::String("e".to_owned())),
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
        use moon_test_utils::pretty_assertions::assert_eq;

        figment::Jail::expect_with(|jail| {
            fs::create_dir_all(jail.directory().join("shared")).unwrap();

            jail.create_file(
                format!("shared/{}", super::CONFIG_TASKS_FILENAME),
                include_str!("../../../../tests/fixtures/config-extends/.moon/tasks.yml"),
            )?;

            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                r#"
extends: ./shared/tasks.yml

fileGroups:
    sources:
        - sources/**/*
    configs:
        - '*.js'
"#,
            )?;

            let config: InheritedTasksConfig = super::load_jailed_config(jail.directory())?;

            // Ensure values are deep merged
            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
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
        use moon_test_utils::pretty_assertions::assert_eq;

        let server = MockServer::start();

        server.mock(|when, then| {
            when.method(GET).path("/config.yml");
            then.status(200).body(include_str!(
                "../../../../tests/fixtures/config-extends/.moon/tasks.yml"
            ));
        });

        let url = server.url("/config.yml");

        figment::Jail::expect_with(|jail| {
            jail.set_env(
                "MOON_WORKSPACE_ROOT",
                jail.directory().to_owned().to_string_lossy(),
            );

            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                format!(
                    r#"
extends: '{url}'

fileGroups:
    sources:
        - sources/**/*
    configs:
        - '*.js'
"#
                )
                .as_ref(),
            )?;

            let config: InheritedTasksConfig = super::load_jailed_config(jail.directory())?;

            // Ensure values are deep merged
            assert_eq!(
                config.file_groups,
                FxHashMap::from_iter([
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
    //                     super::CONFIG_TASKS_FILENAME,
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
        expected = "invalid type: found unsigned int `123`, expected a map for key \"inheritedTasks.fileGroups\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_TASKS_FILENAME, "fileGroups: 123")?;

            super::load_jailed_config(jail.directory())?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a sequence for key \"inheritedTasks.fileGroups.sources\""
    )]
    fn invalid_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
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
    use super::*;
    use moon_config::TaskConfig;

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a map for key \"inheritedTasks.tasks\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
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
        expected = "invalid type: found unsigned int `123`, expected struct TaskConfig for key \"inheritedTasks.tasks.test\""
    )]
    fn invalid_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
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
        expected = "expected a string or a sequence of strings for key \"inheritedTasks.tasks.test.command\""
    )]
    fn invalid_value_field() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
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
                super::CONFIG_TASKS_FILENAME,
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

    #[test]
    fn can_use_references() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                r#"
tasks:
    build: &webpack
        command: 'webpack'
        inputs:
            - 'src/**/*'
    start:
        <<: *webpack
        args: 'serve'
"#,
            )?;

            let config: InheritedTasksConfig = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.tasks.get("build").unwrap(),
                &TaskConfig {
                    command: Some(TaskCommandArgs::String("webpack".to_owned())),
                    inputs: Some(string_vec!["src/**/*"]),
                    ..TaskConfig::default()
                }
            );

            assert_eq!(
                config.tasks.get("start").unwrap(),
                &TaskConfig {
                    command: Some(TaskCommandArgs::String("webpack".to_owned())),
                    args: Some(TaskCommandArgs::String("serve".to_owned())),
                    inputs: Some(string_vec!["src/**/*"]),
                    ..TaskConfig::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    fn can_use_references_from_root() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_TASKS_FILENAME,
                r#"
_webpack: &webpack
    command: 'webpack'
    inputs:
        - 'src/**/*'

tasks:
    build: *webpack
    start:
        <<: *webpack
        args: 'serve'
"#,
            )?;

            let config: InheritedTasksConfig = super::load_jailed_config(jail.directory())?;

            assert_eq!(
                config.tasks.get("build").unwrap(),
                &TaskConfig {
                    command: Some(TaskCommandArgs::String("webpack".to_owned())),
                    inputs: Some(string_vec!["src/**/*"]),
                    ..TaskConfig::default()
                }
            );

            assert_eq!(
                config.tasks.get("start").unwrap(),
                &TaskConfig {
                    command: Some(TaskCommandArgs::String("webpack".to_owned())),
                    args: Some(TaskCommandArgs::String("serve".to_owned())),
                    inputs: Some(string_vec!["src/**/*"]),
                    ..TaskConfig::default()
                }
            );

            Ok(())
        });
    }
}
