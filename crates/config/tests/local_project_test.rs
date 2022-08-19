use moon_config::{
    DependencyConfig, DependencyScope, ProjectConfig, ProjectDependsOn, TaskCommandArgs, TaskConfig,
};
use moon_constants::CONFIG_PROJECT_FILENAME;
use moon_utils::string_vec;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn load_jailed_config() -> Result<ProjectConfig, figment::Error> {
    match ProjectConfig::load(&PathBuf::from(CONFIG_PROJECT_FILENAME)) {
        Ok(cfg) => Ok(cfg),
        Err(errors) => Err(errors.first().unwrap().clone()),
    }
}

#[test]
fn empty_file() {
    figment::Jail::expect_with(|jail| {
        // Needs a fake yaml value, otherwise the file reading panics
        jail.create_file(CONFIG_PROJECT_FILENAME, "fake: value")?;

        load_jailed_config()?;

        Ok(())
    });
}

#[test]
fn loads_defaults() {
    figment::Jail::expect_with(|jail| {
        jail.create_file(
            CONFIG_PROJECT_FILENAME,
            r#"
fileGroups:
    sources:
        - src/**/*"#,
        )?;

        let config = load_jailed_config()?;

        assert_eq!(
            config,
            ProjectConfig {
                file_groups: HashMap::from([(String::from("sources"), string_vec!["src/**/*"])]),
                ..ProjectConfig::default()
            }
        );

        Ok(())
    });
}

mod depends_on {
    use super::*;

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a sequence for key \"project.dependsOn\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "dependsOn: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "expected a project name or dependency config object for key \"project.dependsOn.0\""
    )]
    fn invalid_object_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"dependsOn:
  - id: 'a'
    scope: 'invalid'"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    fn supports_list_of_strings() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "dependsOn: ['a', 'b', 'c']")?;

            let cfg: ProjectConfig = super::load_jailed_config()?;

            assert_eq!(
                cfg.depends_on,
                vec![
                    ProjectDependsOn::String("a".to_owned()),
                    ProjectDependsOn::String("b".to_owned()),
                    ProjectDependsOn::String("c".to_owned())
                ]
            );

            Ok(())
        });
    }

    #[test]
    fn supports_list_of_objects() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"dependsOn:
  - id: 'a'
    scope: 'development'
  - id: 'b'
    scope: 'production'"#,
            )?;

            let cfg: ProjectConfig = super::load_jailed_config()?;

            assert_eq!(
                cfg.depends_on,
                vec![
                    ProjectDependsOn::Object(DependencyConfig {
                        id: "a".to_owned(),
                        scope: DependencyScope::Development
                    }),
                    ProjectDependsOn::Object(DependencyConfig {
                        id: "b".to_owned(),
                        scope: DependencyScope::Production
                    })
                ]
            );

            Ok(())
        });
    }

    #[test]
    fn supports_list_of_strings_and_objects() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"dependsOn:
  - 'a'
  - id: 'b'
    scope: 'production'"#,
            )?;

            let cfg: ProjectConfig = super::load_jailed_config()?;

            assert_eq!(
                cfg.depends_on,
                vec![
                    ProjectDependsOn::String("a".to_owned()),
                    ProjectDependsOn::Object(DependencyConfig {
                        id: "b".to_owned(),
                        scope: DependencyScope::Production
                    })
                ]
            );

            Ok(())
        });
    }
}

mod file_groups {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a map for key \"project.fileGroups\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "fileGroups: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a sequence for key \"project.fileGroups.sources\""
    )]
    fn invalid_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
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
    use super::*;

    // TODO: https://github.com/SergioBenitez/Figment/issues/41
    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                CONFIG_PROJECT_FILENAME,
                r#"
tasks:
    lint:
        command: eslint
        args:
            - ."#,
            )?;

            let config = load_jailed_config()?;

            assert_eq!(
                config,
                ProjectConfig {
                    tasks: BTreeMap::from([(
                        String::from("lint"),
                        TaskConfig {
                            command: Some(TaskCommandArgs::String("eslint".to_owned())),
                            args: Some(TaskCommandArgs::Sequence(vec![".".to_owned()])),
                            ..TaskConfig::default()
                        }
                    )]),
                    ..ProjectConfig::default()
                }
            );

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a map for key \"project.tasks\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "tasks: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct TaskConfig for key \"project.tasks.test\""
    )]
    fn invalid_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
tasks:
    test: 123"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "expected a string or a sequence of strings for key \"project.tasks.test.command\""
    )]
    fn invalid_value_field() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
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
        expected = "An npm/system command is required for key \"project.tasks.test.command\""
    )]
    fn invalid_value_empty_field() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
fileGroups: {}
tasks:
    test:
        command: ''
"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }
}

mod project {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct ProjectMetadataConfig for key \"project.project\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "project: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"project.project.name\""
    )]
    fn invalid_name_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    name: 123
    description: ''
    owner: ''
    maintainers: []
    channel: ''"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found bool true, expected a string for key \"project.project.description\""
    )]
    fn invalid_description_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    name: ''
    description: true
    owner: ''
    maintainers: []
    channel: ''"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found map, expected a string for key \"project.project.owner\""
    )]
    fn invalid_owner_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    name: ''
    description: ''
    owner: {}
    maintainers: []
    channel: ''"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found string \"abc\", expected a sequence for key \"project.project.maintainers\""
    )]
    fn invalid_maintainers_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    name: ''
    description: ''
    owner: ''
    maintainers: 'abc'
    channel: ''"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected a string for key \"project.project.channel\""
    )]
    fn invalid_channel_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    name: ''
    description: ''
    owner: ''
    maintainers: []
    channel: 123"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(expected = "Must start with a `#` for key \"project.project.channel\"")]
    fn channel_leading_hash() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
project:
    type: 'library'
    name: ''
    description: ''
    owner: ''
    maintainers: []
    channel: name"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }
}

mod workspace {
    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct ProjectWorkspaceConfig for key \"project.workspace\""
    )]
    fn invalid_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(super::CONFIG_PROJECT_FILENAME, "workspace: 123")?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found unsigned int `123`, expected struct ProjectWorkspaceInheritedTasksConfig for key \"project.workspace.inheritedTasks\""
    )]
    fn invalid_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
workspace:
    inheritedTasks: 123"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    #[should_panic(
        expected = "invalid type: found string \"abc\", expected a sequence for key \"project.workspace.inheritedTasks.include\""
    )]
    fn invalid_nested_value_type() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                super::CONFIG_PROJECT_FILENAME,
                r#"
workspace:
    inheritedTasks:
        include: abc"#,
            )?;

            super::load_jailed_config()?;

            Ok(())
        });
    }
}
