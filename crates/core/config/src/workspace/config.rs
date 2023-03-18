// .moon/workspace.yml

use crate::errors::map_validation_errors_to_figment_errors;
use crate::errors::ConfigError;
use crate::helpers::{gather_extended_sources, warn_for_unknown_fields};
use crate::types::{FileGlob, FilePath};
use crate::validators::{
    is_default, is_default_true, validate_child_relative_path, validate_extends, validate_id,
    validate_semver_requirement,
};
use crate::workspace::constraints::ConstraintsConfig;
use crate::workspace::generator::GeneratorConfig;
use crate::workspace::hasher::HasherConfig;
use crate::workspace::notifier::NotifierConfig;
use crate::workspace::runner::RunnerConfig;
use crate::workspace::vcs::VcsConfig;
use figment::{
    providers::{Format, Serialized, YamlExtended},
    Figment,
};
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use validator::{Validate, ValidationError};

type ProjectsMap = FxHashMap<String, FilePath>;

// Validate the `projects` field is a map of valid file system paths
// that are relative from the workspace root. Will fail on absolute
// paths ("/"), and parent relative paths ("../").
fn validate_projects(projects: &WorkspaceProjects) -> Result<(), ValidationError> {
    let map = match projects {
        WorkspaceProjects::Sources(sources) => Some(sources),
        WorkspaceProjects::Both { sources, .. } => Some(sources),
        _ => None,
    };

    if let Some(map) = map {
        for (key, value) in map {
            validate_id(format!("projects.{key}"), key)?;

            match validate_child_relative_path("projects", value) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}

fn validate_version_constraint(value: &str) -> Result<(), ValidationError> {
    validate_semver_requirement("versionConstraint", value)?;

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    untagged,
    expecting = "expected a sequence of globs or a map of projects"
)]
pub enum WorkspaceProjects {
    Both {
        globs: Vec<FileGlob>,
        sources: ProjectsMap,
    },
    Globs(Vec<FileGlob>),
    Sources(ProjectsMap),
}

impl Default for WorkspaceProjects {
    fn default() -> Self {
        WorkspaceProjects::Sources(FxHashMap::default())
    }
}

/// Docs: https://moonrepo.dev/docs/config/workspace
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceConfig {
    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub constraints: ConstraintsConfig,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_extends")]
    pub extends: Option<String>,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub generator: GeneratorConfig,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub hasher: HasherConfig,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub notifier: NotifierConfig,

    #[serde(skip_serializing_if = "is_default")]
    #[validate(custom = "validate_projects")]
    pub projects: WorkspaceProjects,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub runner: RunnerConfig,

    #[serde(skip_serializing_if = "is_default_true")]
    pub telemetry: bool,

    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub vcs: VcsConfig,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(custom = "validate_version_constraint")]
    pub version_constraint: Option<String>,

    /// JSON schema URI
    #[serde(rename = "$schema", skip_serializing_if = "is_default")]
    pub schema: String,

    /// Unknown fields
    #[serde(flatten)]
    #[schemars(skip)]
    pub unknown: BTreeMap<String, serde_yaml::Value>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        WorkspaceConfig {
            extends: None,
            generator: GeneratorConfig::default(),
            hasher: HasherConfig::default(),
            notifier: NotifierConfig::default(),
            projects: WorkspaceProjects::default(),
            runner: RunnerConfig::default(),
            telemetry: true,
            vcs: VcsConfig::default(),
            version_constraint: None,
            schema: String::new(),
            unknown: BTreeMap::new(),
        }
    }
}

impl WorkspaceConfig {
    pub fn load(path: PathBuf) -> Result<WorkspaceConfig, ConfigError> {
        let profile_name = "workspace";
        let mut figment =
            Figment::from(Serialized::defaults(WorkspaceConfig::default()).profile(profile_name));

        for source in gather_extended_sources(path)? {
            figment = figment.merge(YamlExtended::file(source).profile(profile_name));
        }

        let mut config = WorkspaceConfig::load_config(figment.select(profile_name))?;
        config.extends = None;

        Ok(config)
    }

    fn load_config(figment: Figment) -> Result<WorkspaceConfig, ConfigError> {
        let config: WorkspaceConfig = figment.extract()?;

        warn_for_unknown_fields(".moon/workspace.yml", &config.unknown);

        if let Err(errors) = config.validate() {
            return Err(ConfigError::FailedValidation(
                map_validation_errors_to_figment_errors(&figment, &errors),
            ));
        }

        Ok(config)
    }
}
