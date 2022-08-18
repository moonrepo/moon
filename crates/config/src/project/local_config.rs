// <project path>/moon.yml

use crate::errors::{create_validation_error, map_validation_errors_to_figment_errors};
use crate::project::dep::DependencyConfig;
use crate::project::task::TaskConfig;
use crate::types::{FileGroups, ProjectID, TaskID};
use crate::validators::{
    skip_if_btree_empty, skip_if_default, skip_if_hash_empty, skip_if_vec_empty, validate_id,
};
use figment::{
    providers::{Format, Serialized, Yaml},
    Error as FigmentError, Figment,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use strum::Display;
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
