// .monolith/project.yml

use crate::constants;
use crate::errors::map_figment_error_to_validation_errors;
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
    file_groups: HashMap<String, Vec<String>>,
}

impl Default for GlobalProjectConfig {
    fn default() -> Self {
        let mut file_groups: HashMap<String, Vec<String>> = HashMap::new();

        file_groups.insert(String::from("configs"), vec![String::from("*.{js,json}")]);

        file_groups.insert(
            String::from("sources"),
            vec![String::from("src/**/*"), String::from("types/**/*")],
        );

        file_groups.insert(
            String::from("tests"),
            vec![
                String::from("tests/**/*.test.*"),
                String::from("**/__tests__/**/*"),
            ],
        );

        file_groups.insert(
            String::from("assets"),
            vec![
                String::from("assets/**/*"),
                String::from("images/**/*"),
                String::from("static/**/*"),
                String::from("**/*.s?css"),
                String::from("**/*.mdx?"),
            ],
        );

        GlobalProjectConfig { file_groups }
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
        // Load and parse the yaml config file using Figment and handle accordingly.
        // Unfortunately this does some "validation", so instead of having 2 validation paths,
        // let's remap to a `validator` error type, so that downstream can handle easily.
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
