// These configs are project-level settings that override those from the workspace!

use crate::types::TaskID;
use crate::validators::validate_semver_version;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::{Validate, ValidationError};

fn validate_node_version(value: &str) -> Result<(), ValidationError> {
    validate_semver_version("workspace.node.version", value)
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct ProjectWorkspaceNodeConfig {
    #[validate(custom = "validate_node_version")]
    pub version: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    pub exclude: Option<Vec<TaskID>>,

    pub include: Option<Vec<TaskID>>,

    pub rename: Option<HashMap<TaskID, TaskID>>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectWorkspaceConfig {
    #[validate]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,

    #[validate]
    pub node: Option<ProjectWorkspaceNodeConfig>,

    pub typescript: bool,
}

impl Default for ProjectWorkspaceConfig {
    fn default() -> Self {
        ProjectWorkspaceConfig {
            inherited_tasks: ProjectWorkspaceInheritedTasksConfig::default(),
            node: None,
            typescript: true,
        }
    }
}
