// .monolith/workspace.yml

use crate::constants;
use crate::errors::map_figment_error_to_validation_errors;
use crate::validators::{validate_child_relative_path, validate_semver_version};
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Yaml},
    Figment, Metadata, Profile, Provider,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use validator::{Validate, ValidationError, ValidationErrors};

const NODE_VERSION: &str = "16.13.0";
const NPM_VERSION: &str = "8.1.0";

fn validate_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("version", value)
}

// Validate the `projects` field is a map of valid file system paths
// that are relative from the workspace root. Will fail on absolute
// paths ("/"), and parent relative paths ("../").
fn validate_projects_map(projects: &HashMap<String, String>) -> Result<(), ValidationError> {
    for value in projects.values() {
        match validate_child_relative_path("projects", value) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_camel_case_types)]
pub enum PackageManager {
    npm,
    pnpm,
    yarn,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct NodeConfig {
    #[validate(custom = "validate_version")]
    pub version: String,

    #[serde(rename = "packageManager")]
    pub package_manager: Option<PackageManager>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            version: String::from(NODE_VERSION),
            package_manager: Some(PackageManager::npm),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct PackageManagerConfig {
    #[validate(custom = "validate_version")]
    pub version: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct WorkspaceConfig {
    #[serde(default)]
    pub node: NodeConfig,

    #[validate(custom = "validate_projects_map")]
    pub projects: HashMap<String, String>,

    // Package managers
    pub npm: Option<PackageManagerConfig>,
    pub pnpm: Option<PackageManagerConfig>,
    pub yarn: Option<PackageManagerConfig>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        WorkspaceConfig {
            node: NodeConfig::default(),
            projects: HashMap::new(),
            npm: None,
            pnpm: None,
            yarn: None,
        }
    }
}

impl Provider for WorkspaceConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named(constants::CONFIG_WORKSPACE_FILENAME)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        figment::providers::Serialized::defaults(WorkspaceConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl WorkspaceConfig {
    pub fn load(path: PathBuf) -> Result<WorkspaceConfig, ValidationErrors> {
        // Load and parse the yaml config file using Figment and handle accordingly.
        // Unfortunately this does some "validation", so instead of having 2 validation paths,
        // let's remap to a `validator` error type, so that downstream can handle easily.
        let mut config: WorkspaceConfig = match Figment::new().merge(Yaml::file(path)).extract() {
            Ok(cfg) => cfg,
            Err(error) => return Err(map_figment_error_to_validation_errors(&error)),
        };

        // We should always have an npm version,
        // as it's also required for installing Yarn and pnpm!
        if config.npm.is_none() {
            config.npm = Some(PackageManagerConfig {
                version: String::from(NPM_VERSION),
            });
        }

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
    use crate::errors::format_validation_error;
    use figment;

    fn load_jailed_config() -> Result<WorkspaceConfig, figment::Error> {
        match WorkspaceConfig::load(PathBuf::from(constants::CONFIG_WORKSPACE_FILENAME)) {
            Ok(cfg) => return Ok(cfg),
            Err(errors) => {
                let field_errors = errors.field_errors();
                let error_list = field_errors.values().next().unwrap();

                panic!("{}", format_validation_error(error_list.first().unwrap()));

                // return Err(figment::Error::from(figment::error::Kind::Message(
                //     format_validation_error(error_list.first().unwrap()),
                // )));
            }
        }
    }

    #[test]
    #[should_panic(expected = "Missing field `projects`.")]
    fn empty_file() {
        figment::Jail::expect_with(|jail| {
            // Needs a fake yaml value, otherwise the file reading panics
            jail.create_file(constants::CONFIG_WORKSPACE_FILENAME, "fake: value")?;

            load_jailed_config()?;

            Ok(())
        });
    }

    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(constants::CONFIG_WORKSPACE_FILENAME, "projects: {}")?;

            let config = load_jailed_config()?;

            assert_eq!(
                config,
                WorkspaceConfig {
                    node: NodeConfig {
                        version: String::from(NODE_VERSION),
                        package_manager: Some(PackageManager::npm),
                    },
                    projects: HashMap::new(),
                    npm: Some(PackageManagerConfig {
                        version: String::from(NPM_VERSION),
                    }),
                    pnpm: None,
                    yarn: None
                }
            );

            Ok(())
        });
    }

    mod node {
        #[test]
        #[should_panic(
            expected = "Invalid field `node`. Expected struct NodeConfig type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_WORKSPACE_FILENAME, "node: 123")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }

    mod npm {
        #[test]
        #[should_panic(
            expected = "Invalid field `npm`. Expected struct PackageManagerConfig type, received string \"foo\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_WORKSPACE_FILENAME, "npm: foo")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        // #[should_panic(
        //     expected = "Invalid type for field `projects`. Expected a sequence, received string \"apps/*\"."
        // )]
        fn invalid_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
npm:
  version: 'foo bar'
projects:
  foo: packages/foo"#,
                )?;

                let config = super::load_jailed_config()?;

                println!("{:?}", config);

                Ok(())
            });
        }
    }

    mod projects {
        use std::collections::HashMap;

        #[test]
        #[should_panic(
            expected = "Invalid field `projects`. Expected a map type, received string \"apps/*\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    "projects: apps/*",
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(expected = "Invalid field `projects`. Absolute paths are not supported.")]
        fn no_abs_paths() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects:
  app: /apps/app
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field `projects`. Parent relative paths are not supported."
        )]
        fn no_parent_paths() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects:
  app: ../apps/app
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn valid_list() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects:
  app: apps/app
  foo: ./packages/foo"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config.projects,
                    HashMap::from([
                        (String::from("app"), String::from("apps/app")),
                        (String::from("foo"), String::from("./packages/foo"))
                    ]),
                );

                Ok(())
            });
        }
    }
}
