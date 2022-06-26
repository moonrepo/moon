// <project path>/project.yml

pub mod global;
pub mod task;

use crate::constants;
use crate::errors::{create_validation_error, map_figment_error_to_validation_errors};
use crate::types::{FileGroups, ProjectID, TaskID};
use crate::validators::validate_id;
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment, Metadata, Profile, Provider,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use task::TaskConfig;
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

        // Only fail for empty strings and not `None`
        if let Some(command) = &task.command {
            if command.is_empty() {
                return Err(create_validation_error(
                    "required_command",
                    &format!("tasks.{}.command", name),
                    String::from("An npm/system command is required."),
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
            String::from("Must start with a #."),
        ));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectLanguage {
    JavaScript,
    TypeScript,
    Unknown,
}

impl Default for ProjectLanguage {
    fn default() -> Self {
        ProjectLanguage::TypeScript
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Application,
    Library,
    Tool,
    Unknown,
}

impl Default for ProjectType {
    fn default() -> Self {
        ProjectType::Library
    }
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct ProjectMetadataConfig {
    pub name: String,

    pub description: String,

    pub owner: String,

    pub maintainers: Vec<String>,

    #[validate(custom = "validate_channel")]
    pub channel: String,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    pub exclude: Option<Vec<TaskID>>,

    pub include: Option<Vec<TaskID>>,

    pub rename: HashMap<TaskID, TaskID>,
}

#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectWorkspaceConfig {
    #[validate]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
}

/// Docs: https://moonrepo.dev/docs/config/project
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfig {
    pub depends_on: Vec<ProjectID>,

    #[validate(custom = "validate_file_groups")]
    pub file_groups: FileGroups,

    pub language: ProjectLanguage,

    #[validate]
    pub project: Option<ProjectMetadataConfig>,

    #[validate(custom = "validate_tasks")]
    #[validate]
    pub tasks: HashMap<String, TaskConfig>,

    #[serde(rename = "type")]
    pub type_of: ProjectType,

    #[validate]
    pub workspace: ProjectWorkspaceConfig,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl Provider for ProjectConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("Project config").source(constants::CONFIG_PROJECT_FILENAME)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        Serialized::defaults(self).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<ProjectConfig, FigmentError> {
        let config: ProjectConfig = Figment::from(ProjectConfig::default())
            .merge(Yaml::file(path))
            .extract()?;

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
    use moon_utils::string_vec;
    use std::path::PathBuf;

    fn load_jailed_config() -> Result<ProjectConfig, figment::Error> {
        match ProjectConfig::load(&PathBuf::from(constants::CONFIG_PROJECT_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(handled_jailed_error(&errors)),
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
        #[test]
        #[should_panic(
            expected = "Invalid field <id>dependsOn</id>: Expected a sequence type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_PROJECT_FILENAME, "dependsOn: 123")?;

                super::load_jailed_config()?;

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

                super::load_jailed_config()?;

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
                        tasks: HashMap::from([(
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
            expected = "Invalid field <id>tasks</id>: Expected a map type, received unsigned int `123`."
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
            expected = "Invalid field <id>tasks.test</id>: Expected struct TaskConfig type, received unsigned int `123`."
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

                super::load_jailed_config()?;

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
            expected = "Invalid field <id>project</id>: Expected struct ProjectMetadataConfig type, received unsigned int `123`."
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
            expected = "Invalid field <id>project.name</id>: Expected a string type, received unsigned int `123`."
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
            expected = "Invalid field <id>project.description</id>: Expected a string type, received bool true."
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
            expected = "Invalid field <id>project.owner</id>: Expected a string type, received map."
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
            expected = "Invalid field <id>project.maintainers</id>: Expected a sequence type, received string \"abc\"."
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
            expected = "Invalid field <id>project.channel</id>: Expected a string type, received unsigned int `123`."
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
        #[should_panic(expected = "Invalid field <id>project.channel</id>: Must start with a #.")]
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
            expected = "Invalid field <id>workspace</id>: Expected struct ProjectWorkspaceConfig type, received unsigned int `123`."
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
            expected = "Invalid field <id>workspace.inheritedTasks</id>: Expected struct ProjectWorkspaceInheritedTasksConfig type, received unsigned int `123`."
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
            expected = "Invalid field <id>workspace.inheritedTasks.include</id>: Expected a sequence type, received string \"abc\"."
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
