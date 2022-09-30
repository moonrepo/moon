// These configs are project-level settings that override those from the workspace!

use crate::types::TaskID;
use crate::validators::skip_if_default;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

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
