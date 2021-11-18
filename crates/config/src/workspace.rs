// .monolith/workspace.yml

use crate::constants;
use crate::errors::{create_validation_error, map_figment_error_to_validation_errors};
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Yaml},
    Figment, Metadata, Profile, Provider,
};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use validator::{Validate, ValidationError, ValidationErrors};

const NODE_VERSION: &str = "16.13.0";
const NPM_VERSION: &str = "8.1.0";

pub fn validate_version(value: &str) -> Result<(), ValidationError> {
    if Version::parse(value).is_err() {
        return Err(create_validation_error(
            "invalid_semver",
            "version",
            String::from("Must be valid semver."),
        ));
    }

    Ok(())
}

// Validate the `projects` field is a list of valid file system globs,
// that are relative from the workspace root. Will fail on absolute
// globs ("/"), and parent relative globs ("../").
fn validate_projects_list(projects: &[String]) -> Result<(), ValidationError> {
    for path_glob in projects {
        let path = Path::new(path_glob);

        if path.has_root() || path.is_absolute() {
            return Err(create_validation_error(
                "no_root",
                "projects",
                String::from("Absolute paths are not supported."),
            ));
        } else if path.starts_with("..") {
            return Err(create_validation_error(
                "no_parent",
                "projects",
                String::from("Parent relative paths are not supported."),
            ));
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
pub struct NodeConfigShasums {
    #[validate(length(min = 1))]
    pub linux: Option<Vec<String>>,

    #[validate(length(min = 1))]
    pub macos: Option<Vec<String>>,

    #[validate(length(min = 1))]
    pub windows: Option<Vec<String>>,
}

impl Default for NodeConfigShasums {
    fn default() -> Self {
        // https://nodejs.org/dist/v16.13.0/SHASUMS256.txt.asc
        NodeConfigShasums {
            linux: Some(vec![
                // linux-arm64
                String::from("46e3857f5552abd36d9548380d795b043a3ceec2504e69fe1a754fa76012daaf"),
                // linux-x64
                String::from("589b7e7eb22f8358797a2c14a0bd865459d0b44458b8f05d2721294dacc7f734"),
            ]),
            macos: Some(vec![
                // darwin-arm64
                String::from("46d83fc0bd971db5050ef1b15afc44a6665dee40bd6c1cbaec23e1b40fa49e6d"),
                // darwin-x64
                String::from("37e09a8cf2352f340d1204c6154058d81362fef4ec488b0197b2ce36b3f0367a"),
            ]),
            windows: Some(vec![
                // x64
                String::from("bf55b68293b163423ea4856c1d330be23158e78aea18a8756cfdff6fb6ffcd88"),
            ]),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize, Validate)]
pub struct NodeConfig {
    #[validate(custom = "validate_version")]
    pub version: String,

    #[serde(rename = "packageManager")]
    pub package_manager: Option<PackageManager>,

    #[serde(default)]
    pub shasums: NodeConfigShasums,
}

impl Default for NodeConfig {
    fn default() -> Self {
        NodeConfig {
            version: String::from(NODE_VERSION),
            package_manager: Some(PackageManager::npm),
            shasums: NodeConfigShasums::default(),
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

    #[validate(custom = "validate_projects_list")]
    pub projects: Vec<String>,

    // Package managers
    pub npm: Option<PackageManagerConfig>,
    pub pnpm: Option<PackageManagerConfig>,
    pub yarn: Option<PackageManagerConfig>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        WorkspaceConfig {
            node: NodeConfig::default(),
            projects: vec![],
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
            jail.create_file(constants::CONFIG_WORKSPACE_FILENAME, "projects: []")?;

            let config = load_jailed_config()?;

            assert_eq!(
                config,
                WorkspaceConfig {
                    node: NodeConfig {
                        version: String::from(NODE_VERSION),
                        package_manager: Some(PackageManager::npm),
                        shasums: NodeConfigShasums::default(),
                    },
                    projects: vec![],
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
  - 'packages/*'"#,
                )?;

                let config = super::load_jailed_config()?;

                println!("{:?}", config);

                Ok(())
            });
        }
    }

    mod projects {
        #[test]
        #[should_panic(
            expected = "Invalid field `projects`. Expected a sequence type, received string \"apps/*\"."
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
  - '/apps/*'
  - 'packages/*'"#,
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
  - '../apps/*'
  - 'packages/*'"#,
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
  - 'apps/*'
  - './packages/*'"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config.projects,
                    vec![String::from("apps/*"), String::from("./packages/*")],
                );

                Ok(())
            });
        }
    }
}
