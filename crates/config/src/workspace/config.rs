// .moon/workspace.yml

use crate::errors::map_validation_errors_to_figment_errors;
use crate::helpers::gather_extended_sources;
use crate::providers::url::Url;
use crate::types::{FileGlob, FilePath};
use crate::validators::{validate_child_relative_path, validate_extends, validate_id};
use crate::workspace::generator::GeneratorConfig;
use crate::workspace::hasher::HasherConfig;
use crate::workspace::node::NodeConfig;
use crate::workspace::notifier::NotifierConfig;
use crate::workspace::runner::RunnerConfig;
use crate::workspace::typescript::TypeScriptConfig;
use crate::workspace::vcs::VcsConfig;
use crate::ConfigError;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use validator::{Validate, ValidationError};

type ProjectsMap = HashMap<String, FilePath>;

// Validate the `projects` field is a map of valid file system paths
// that are relative from the workspace root. Will fail on absolute
// paths ("/"), and parent relative paths ("../").
fn validate_projects(projects: &WorkspaceProjects) -> Result<(), ValidationError> {
    if let WorkspaceProjects::Map(map) = projects {
        for (key, value) in map {
            validate_id(format!("projects.{}", key), key)?;

            match validate_child_relative_path("projects", value) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    untagged,
    expecting = "expected a sequence of globs or a map of projects"
)]
pub enum WorkspaceProjects {
    List(Vec<FileGlob>),
    Map(ProjectsMap),
}

impl Default for WorkspaceProjects {
    fn default() -> Self {
        WorkspaceProjects::Map(HashMap::new())
    }
}

/// Docs: https://moonrepo.dev/docs/config/workspace
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceConfig {
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[validate]
    pub generator: GeneratorConfig,

    #[validate]
    pub hasher: HasherConfig,

    #[validate]
    pub node: Option<NodeConfig>,

    #[validate]
    pub notifier: NotifierConfig,

    #[validate(custom = "validate_projects")]
    pub projects: WorkspaceProjects,

    #[validate]
    pub runner: RunnerConfig,

    #[validate]
    pub typescript: Option<TypeScriptConfig>,

    #[validate]
    pub vcs: VcsConfig,

    /// JSON schema URI.
    #[serde(skip, rename = "$schema")]
    pub schema: String,
}

impl WorkspaceConfig {
    pub fn load(path: PathBuf) -> Result<WorkspaceConfig, ConfigError> {
        let profile_name = "workspace";
        let mut figment =
            Figment::from(Serialized::defaults(WorkspaceConfig::default()).profile(&profile_name));

        for source in gather_extended_sources(&path)? {
            if source.starts_with("http") {
                figment = figment.merge(Url::from(source).profile(&profile_name));
            } else {
                figment = figment.merge(YamlExtended::file(source).profile(&profile_name));
            };
        }

        let mut config = WorkspaceConfig::load_config(figment.select(&profile_name))?;
        config.extends = None;

        if let Some(node_config) = &mut config.node {
            // Versions from env vars should take precedence
            if let Ok(node_version) = env::var("MOON_NODE_VERSION") {
                node_config.version = node_version;
            }

            if let Ok(npm_version) = env::var("MOON_NPM_VERSION") {
                node_config.npm.version = npm_version;
            }

            if let Ok(pnpm_version) = env::var("MOON_PNPM_VERSION") {
                if let Some(pnpm_config) = &mut node_config.pnpm {
                    pnpm_config.version = pnpm_version;
                }
            }

            if let Ok(yarn_version) = env::var("MOON_YARN_VERSION") {
                if let Some(yarn_config) = &mut node_config.yarn {
                    yarn_config.version = yarn_version;
                }
            }
        }

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<WorkspaceConfig, ConfigError> {
        let config: WorkspaceConfig = figment.extract()?;

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
