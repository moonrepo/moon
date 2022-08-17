// <project path>/moon.yml

pub mod dep;
pub mod global;
pub mod task;
pub mod task_options;

use crate::errors::{create_validation_error, map_validation_errors_to_figment_errors};
use crate::types::{FileGroups, ProjectID, TaskID};
use crate::validators::{
    skip_if_btree_empty, skip_if_default, skip_if_hash_empty, skip_if_vec_empty, validate_id,
};
use dep::DependencyConfig;
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use strum::Display;
use task::TaskConfig;
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

        // Only fail for empty strings and not `None`
        if let Some(command) = &task.command {
            if command.is_empty() {
                return Err(create_validation_error(
                    "required_command",
                    &format!("tasks.{}.command", name),
                    String::from("An npm/system command is required"),
                ));
            }
        }
    }

    Ok(())
}

fn validate_channel(value: &str) -> Result<(), ValidationError> {
    if !value.is_empty() && !value.starts_with('#') {
        return Err(create_validation_error(
            "invalid_channel",
            "project.channel",
            String::from("Must start with a `#`"),
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectLanguage {
    #[strum(serialize = "bash")]
    Bash,

    #[strum(serialize = "batch")]
    Batch,

    #[strum(serialize = "javascript")]
    JavaScript,

    #[strum(serialize = "typescript")]
    TypeScript,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Display, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    #[strum(serialize = "application")]
    Application,

    #[strum(serialize = "library")]
    Library,

    #[strum(serialize = "tool")]
    Tool,

    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
pub struct ProjectMetadataConfig {
    pub name: String,

    pub description: String,

    pub owner: String,

    pub maintainers: Vec<String>,

    #[validate(custom = "validate_channel")]
    pub channel: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<TaskID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<TaskID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename: Option<HashMap<TaskID, TaskID>>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectWorkspaceConfig {
    #[serde(skip_serializing_if = "skip_if_default")]
    #[validate]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,

    pub typescript: bool,
}

impl Default for ProjectWorkspaceConfig {
    fn default() -> Self {
        ProjectWorkspaceConfig {
            inherited_tasks: ProjectWorkspaceInheritedTasksConfig::default(),
            typescript: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    untagged,
    expecting = "expected a project name or dependency config object"
)]
pub enum ProjectDependsOn {
    String(ProjectID),
    Object(DependencyConfig),
}

/// Docs: https://moonrepo.dev/docs/config/project
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectConfig {
    #[serde(skip_serializing_if = "skip_if_vec_empty")]
    pub depends_on: Vec<ProjectDependsOn>,

    #[serde(skip_serializing_if = "skip_if_hash_empty")]
    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    #[serde(skip_serializing_if = "skip_if_default")]
    pub language: ProjectLanguage,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate]
    pub project: Option<ProjectMetadataConfig>,

    #[serde(skip_serializing_if = "skip_if_btree_empty")]
    #[validate(custom = "validate_tasks")]
    #[validate]
    pub tasks: BTreeMap<String, TaskConfig>,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[serde(rename = "type")]
    pub type_of: ProjectType,

    #[serde(skip_serializing_if = "skip_if_default")]
    #[validate]
    pub workspace: ProjectWorkspaceConfig,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl ProjectConfig {
    pub fn detect_language<T: AsRef<Path>>(root: T) -> ProjectLanguage {
        let root = root.as_ref();

        if root.join("tsconfig.json").exists() {
            ProjectLanguage::TypeScript
        } else if root.join("package.json").exists() {
            ProjectLanguage::JavaScript
        } else {
            ProjectLanguage::Unknown
        }
    }

    #[track_caller]
    pub fn load<T: AsRef<Path>>(path: T) -> Result<ProjectConfig, Vec<FigmentError>> {
        let path = path.as_ref();
        let profile_name = "project";
        let figment =
            Figment::from(Serialized::defaults(ProjectConfig::default()).profile(&profile_name))
                .merge(Yaml::file(path).profile(&profile_name))
                .select(&profile_name);

        let mut config: ProjectConfig = figment.extract().map_err(|e| vec![e])?;

        if let Err(errors) = config.validate() {
            return Err(map_validation_errors_to_figment_errors(&figment, &errors));
        }

        if matches!(config.language, ProjectLanguage::Unknown) {
            config.language = ProjectConfig::detect_language(path.parent().unwrap());
        }

        Ok(config)
    }

    pub fn new<T: AsRef<Path>>(root: T) -> Self {
        ProjectConfig {
            language: ProjectConfig::detect_language(root.as_ref()),
            ..ProjectConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::dep::DependencyScope;
    use moon_constants as constants;
    use moon_utils::string_vec;
    use std::path::PathBuf;

    fn load_jailed_config() -> Result<ProjectConfig, figment::Error> {
        match ProjectConfig::load(&PathBuf::from(constants::CONFIG_PROJECT_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(errors.first().unwrap().clone()),
        }
    }

    #[test]
    fn empty_file() {
        figment::Jail::expect_with(|jail| {
            // Needs a fake yaml value, otherwise the file reading panics
            jail.create_file(constants::CONFIG_PROJECT_FILENAME, "fake: value")?;

            load_jailed_config()?;

            Ok(())
        });
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
                ProjectConfig {
                    file_groups: HashMap::from([(
                        String::from("sources"),
                        string_vec!["src/**/*"]
                    )]),
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
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "dependsOn: 123")?;

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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    "dependsOn: ['a', 'b', 'c']",
                )?;

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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "fileGroups: 123")?;

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
        use super::*;

        // TODO: https://github.com/SergioBenitez/Figment/issues/41
        #[test]
        fn loads_defaults() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    constants::CONFIG_PROJECT_FILENAME,
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
                                args: Some(vec![".".to_owned()]),
                                command: Some("eslint".to_owned()),
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
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "tasks: 123")?;

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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
            expected = "invalid type: found unsigned int `123`, expected a string for key \"project.tasks.test.command\""
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
            expected = "An npm/system command is required for key \"project.tasks.test.command\""
        )]
        fn invalid_value_empty_field() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "project: 123")?;

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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "workspace: 123")?;

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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
                    super::constants::CONFIG_PROJECT_FILENAME,
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
}
