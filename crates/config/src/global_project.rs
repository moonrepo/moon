// .monolith/project.yml

use crate::constants;
use crate::errors::map_figment_error_to_validation_errors;
use crate::project::FileGroups;
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Yaml},
    Figment, Metadata, Profile, Provider,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct GlobalProjectConfig {
    #[serde(rename = "fileGroups")]
    pub file_groups: FileGroups,
}

impl Default for GlobalProjectConfig {
    fn default() -> Self {
        GlobalProjectConfig {
            file_groups: HashMap::new(),
        }
    }
}

impl Provider for GlobalProjectConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named(constants::CONFIG_PROJECT_FILENAME)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        figment::providers::Serialized::defaults(GlobalProjectConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl GlobalProjectConfig {
    pub fn load(path: PathBuf) -> Result<GlobalProjectConfig, ValidationErrors> {
        let config: GlobalProjectConfig = match Figment::new().merge(Yaml::file(path)).extract() {
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

    fn load_jailed_config() -> Result<GlobalProjectConfig, figment::Error> {
        match GlobalProjectConfig::load(PathBuf::from(constants::CONFIG_PROJECT_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(handled_jailed_error(&errors)),
        }
    }

    #[test]
    #[should_panic(expected = "Missing field `fileGroups`.")]
    fn empty_file() {
        figment::Jail::expect_with(|jail| {
            // Needs a fake yaml value, otherwise the file reading panics
            jail.create_file(constants::CONFIG_PROJECT_FILENAME, "fake: value")?;

            load_jailed_config()?;

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
}
