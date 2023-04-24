// These configs are project-level settings that override those from the workspace!

use crate::types::TaskID;
use crate::validators::is_default;
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<TaskID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<TaskID>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rename: Option<FxHashMap<TaskID, TaskID>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ProjectWorkspaceConfig {
    #[serde(skip_serializing_if = "is_default")]
    #[validate]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
}
