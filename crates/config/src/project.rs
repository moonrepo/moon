// <project path>/project.yml

use crate::constants;
use crate::errors::{create_validation_error, map_figment_error_to_validation_errors};
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Yaml},
    Figment, Metadata, Profile, Provider,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use validator::{Validate, ValidationError, ValidationErrors};

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

pub type FileGroups = HashMap<String, Vec<String>>;

pub type ProjectID = String;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct ProjectMetadataConfig {
    pub name: String,

    pub description: String,

    pub owner: String,

    pub maintainers: Vec<String>,

    #[validate(custom = "validate_channel")]
    pub channel: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct ProjectConfig {
    #[serde(rename = "dependsOn")]
    pub depends_on: Option<Vec<ProjectID>>,

    #[serde(rename = "fileGroups")]
    pub file_groups: Option<FileGroups>,

    #[validate]
    pub project: Option<ProjectMetadataConfig>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        ProjectConfig {
            depends_on: None,
            file_groups: None,
            project: None,
        }
    }
}

impl Provider for ProjectConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named(constants::CONFIG_PROJECT_FILENAME)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        figment::providers::Serialized::defaults(ProjectConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl ProjectConfig {
    pub fn load(path: &Path) -> Result<ProjectConfig, ValidationErrors> {
        let config: ProjectConfig = match Figment::new().merge(Yaml::file(path)).extract() {
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

    mod depends_on {
        #[test]
        #[should_panic(
            expected = "Invalid field `dependsOn`. Expected a sequence type, received unsigned int `123`."
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

    mod project {
        #[test]
        #[should_panic(
            expected = "Invalid field `project`. Expected struct ProjectMetadataConfig type, received unsigned int `123`."
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
            expected = "Invalid field `project.name`. Expected a string type, received unsigned int `123`."
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
            expected = "Invalid field `project.description`. Expected a string type, received bool true."
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
            expected = "Invalid field `project.owner`. Expected a string type, received map."
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
            expected = "Invalid field `project.maintainers`. Expected a sequence type, received string \"abc\"."
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
            expected = "Invalid field `project.channel`. Expected a string type, received unsigned int `123`."
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
        #[should_panic(expected = "Invalid field `project.channel`. Must start with a #.")]
        fn channel_leading_hash() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_PROJECT_FILENAME,
                    r#"
project:
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
}
