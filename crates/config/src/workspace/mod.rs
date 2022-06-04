// .moon/workspace.yml

pub mod node;
mod typescript;
mod vcs;

use crate::constants;
use crate::errors::map_figment_error_to_validation_errors;
use crate::types::{FileGlob, FilePath};
use crate::validators::{validate_child_relative_path, validate_id};
use figment::value::{Dict, Map};
use figment::{
    providers::{Format, Serialized, Yaml},
    Figment, Metadata, Profile, Provider,
};
pub use node::{NodeConfig, NpmConfig, PackageManager, PnpmConfig, YarnConfig};
use schemars::JsonSchema;
use serde::de::{self, MapAccess, SeqAccess};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::path::PathBuf;
pub use typescript::TypeScriptConfig;
use validator::{Validate, ValidationError, ValidationErrors};
pub use vcs::{VcsConfig, VcsManager};

const FLAG_PROJECTS_USING_GLOB: &str = "MOON_PROJECTS_USING_GLOBS";

type ProjectsMap = HashMap<String, FilePath>;

// Validate the `projects` field is a map of valid file system paths
// that are relative from the workspace root. Will fail on absolute
// paths ("/"), and parent relative paths ("../").
fn validate_projects(projects: &ProjectsMap) -> Result<(), ValidationError> {
    for (key, value) in projects {
        validate_id(&format!("projects.{}", key), key)?;

        match validate_child_relative_path("projects", value) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// Docs: https://moonrepo.dev/docs/config/workspace
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, PartialEq, Serialize, Validate)]
pub struct WorkspaceConfig {
    #[serde(default)]
    #[validate]
    pub node: NodeConfig,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_projects")]
    #[validate(custom = "validate_projects")]
    pub projects: ProjectsMap,

    #[serde(default)]
    #[validate]
    pub typescript: TypeScriptConfig,

    #[serde(default)]
    #[validate]
    pub vcs: VcsConfig,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

// SERDE

#[derive(JsonSchema)]
#[serde(untagged)]
enum ProjectsField {
    #[allow(dead_code)]
    Map(ProjectsMap),
    #[allow(dead_code)]
    Globs(Vec<FileGlob>),
}

struct DeserializeProjects;

impl<'de> de::Visitor<'de> for DeserializeProjects {
    type Value = ProjectsMap;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence of project globs or a map of projects")
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut map = HashMap::with_capacity(visitor.size_hint().unwrap_or(0));

        while let Some((key, value)) = visitor.next_entry()? {
            map.insert(key, value);
        }

        Ok(map)
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut map = HashMap::new();
        let mut index: u8 = 0;

        while let Some(elem) = visitor.next_element()? {
            map.insert(index.to_string(), elem);
            index += 1;
        }

        // We want to defer globbing so that we can cache it through
        // the engine, so we must fake this here until config resolving
        // has completed. Annoying, but a serde limitation.
        map.insert(FLAG_PROJECTS_USING_GLOB.to_owned(), "true".to_owned());

        Ok(map)
    }
}

fn deserialize_projects<'de, D>(deserializer: D) -> Result<ProjectsMap, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(DeserializeProjects)
}

impl Provider for WorkspaceConfig {
    fn metadata(&self) -> Metadata {
        Metadata::named("Workspace config").source(format!(
            "{}/{}",
            constants::CONFIG_DIRNAME,
            constants::CONFIG_WORKSPACE_FILENAME
        ))
    }

    fn data(&self) -> Result<Map<Profile, Dict>, figment::Error> {
        Serialized::defaults(WorkspaceConfig::default()).data()
    }

    fn profile(&self) -> Option<Profile> {
        Some(Profile::Default)
    }
}

impl WorkspaceConfig {
    pub fn load(path: PathBuf) -> Result<WorkspaceConfig, ValidationErrors> {
        let mut config: WorkspaceConfig =
            match Figment::from(Serialized::defaults(WorkspaceConfig::default()))
                .merge(Yaml::file(path))
                .extract()
            {
                Ok(cfg) => cfg,
                Err(error) => return Err(map_figment_error_to_validation_errors(&error)),
            };

        if let Err(errors) = config.validate() {
            return Err(errors);
        }

        // Versions from env vars should take precedence
        if let Ok(node_version) = env::var("MOON_NODE_VERSION") {
            config.node.version = node_version;
        }

        if let Ok(npm_version) = env::var("MOON_NPM_VERSION") {
            config.node.npm.version = npm_version;
        }

        if let Ok(pnpm_version) = env::var("MOON_PNPM_VERSION") {
            if let Some(pnpm_config) = &mut config.node.pnpm {
                pnpm_config.version = pnpm_version;
            }
        }

        if let Ok(yarn_version) = env::var("MOON_YARN_VERSION") {
            if let Some(yarn_config) = &mut config.node.yarn {
                yarn_config.version = yarn_version;
            }
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::tests::handled_jailed_error;

    fn load_jailed_config() -> Result<WorkspaceConfig, figment::Error> {
        match WorkspaceConfig::load(PathBuf::from(constants::CONFIG_WORKSPACE_FILENAME)) {
            Ok(cfg) => Ok(cfg),
            Err(errors) => Err(handled_jailed_error(&errors)),
        }
    }

    #[test]
    fn loads_defaults() {
        figment::Jail::expect_with(|jail| {
            jail.create_file(constants::CONFIG_WORKSPACE_FILENAME, "projects: {}")?;

            let config = load_jailed_config()?;

            assert_eq!(
                config,
                WorkspaceConfig {
                    node: NodeConfig::default(),
                    projects: HashMap::new(),
                    typescript: TypeScriptConfig::default(),
                    vcs: VcsConfig::default(),
                    schema: String::new(),
                }
            );

            Ok(())
        });
    }

    mod node {
        use super::*;

        #[test]
        fn loads_defaults() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects: {}
node:
    packageManager: yarn"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config,
                    WorkspaceConfig {
                        node: NodeConfig {
                            package_manager: PackageManager::Yarn,
                            ..NodeConfig::default()
                        },
                        projects: HashMap::new(),
                        typescript: TypeScriptConfig::default(),
                        vcs: VcsConfig::default(),
                        schema: String::new(),
                    }
                );

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node</id>: Expected struct NodeConfig type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(super::constants::CONFIG_WORKSPACE_FILENAME, "node: 123")?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.version</id>: Must be a valid semantic version."
        )]
        fn invalid_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
  version: 'foo bar'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.version</id>: Must be a valid semantic version."
        )]
        fn no_patch_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
  version: '16.13'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.version</id>: Must be a valid semantic version."
        )]
        fn no_minor_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
  version: '16'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.packageManager</id>: Unknown option <id>what</id>."
        )]
        fn invalid_package_manager() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
  version: '16.13.0'
  packageManager: what
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn valid_package_manager() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
  version: '16.13.0'
  packageManager: yarn
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn inherits_from_env_var() {
            std::env::set_var("MOON_NODE_VERSION", "4.5.6");

            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
projects: {}"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(config.node.version, String::from("4.5.6"),);

                Ok(())
            });
        }
    }

    mod npm {
        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.npm</id>: Expected struct NpmConfig type, received string \"foo\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    npm: foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.npm.version</id>: Must be a valid semantic version."
        )]
        fn invalid_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    npm:
        version: 'foo bar'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn inherits_from_env_var() {
            std::env::set_var("MOON_NPM_VERSION", "4.5.6");

            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    npm:
        version: '1.2.3'
projects: {}"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(config.node.npm.version, String::from("4.5.6"),);

                Ok(())
            });
        }
    }

    mod pnpm {
        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.pnpm</id>: Expected struct PnpmConfig type, received string \"foo\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    pnpm: foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.pnpm.version</id>: Must be a valid semantic version."
        )]
        fn invalid_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    pnpm:
        version: 'foo bar'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn inherits_from_env_var() {
            std::env::set_var("MOON_PNPM_VERSION", "4.5.6");

            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    packageManager: 'pnpm'
    pnpm:
        version: '1.2.3'
projects: {}"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(config.node.pnpm.unwrap().version, String::from("4.5.6"),);

                Ok(())
            });
        }
    }

    mod yarn {
        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.yarn</id>: Expected struct YarnConfig type, received string \"foo\"."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    yarn: foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>node.yarn.version</id>: Must be a valid semantic version."
        )]
        fn invalid_version() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    yarn:
        version: 'foo bar'
projects:
  foo: packages/foo"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        fn inherits_from_env_var() {
            std::env::set_var("MOON_YARN_VERSION", "4.5.6");

            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
node:
    version: '16.13.0'
    packageManager: 'yarn'
    yarn:
        version: '1.2.3'
projects: {}"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(config.node.yarn.unwrap().version, String::from("4.5.6"),);

                Ok(())
            });
        }
    }

    mod projects {
        use std::collections::HashMap;

        #[test]
        #[should_panic(
            expected = "Invalid field <id>projects</id>: Expected a map type, received string \"apps/*\"."
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
        #[should_panic(
            expected = "Invalid field <id>projects</id>: Absolute paths are not supported."
        )]
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
            expected = "Invalid field <id>projects</id>: Parent relative paths are not supported."
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

    mod vcs {
        use super::*;

        #[test]
        fn loads_defaults() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects: {}
vcs:
    manager: svn"#,
                )?;

                let config = super::load_jailed_config()?;

                assert_eq!(
                    config,
                    WorkspaceConfig {
                        node: NodeConfig::default(),
                        projects: HashMap::new(),
                        typescript: TypeScriptConfig::default(),
                        vcs: VcsConfig {
                            manager: VcsManager::Svn,
                            ..VcsConfig::default()
                        },
                        schema: String::new(),
                    }
                );

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>vcs</id>: Expected struct VcsConfig type, received unsigned int `123`."
        )]
        fn invalid_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects: {}
vcs: 123"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>vcs.manager</id>: Unknown option <id>unknown</id>."
        )]
        fn invalid_manager_option() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects: {}
vcs:
    manager: unknown"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }

        #[test]
        #[should_panic(
            expected = "Invalid field <id>vcs.defaultBranch</id>: Expected a string type, received unsigned int `123`."
        )]
        fn invalid_default_branch_type() {
            figment::Jail::expect_with(|jail| {
                jail.create_file(
                    super::constants::CONFIG_WORKSPACE_FILENAME,
                    r#"
projects: {}
vcs:
    defaultBranch: 123"#,
                )?;

                super::load_jailed_config()?;

                Ok(())
            });
        }
    }
}
