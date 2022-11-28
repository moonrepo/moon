// These configs are project-level settings that override those from the workspace!

use crate::types::TaskID;
use rustc_hash::FxHashMap;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default)]
pub struct ProjectWorkspaceInheritedTasksConfig {
    pub exclude: Option<Vec<TaskID>>,

    pub include: Option<Vec<TaskID>>,

    pub rename: Option<FxHashMap<TaskID, TaskID>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize, Validate)]
#[schemars(default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProjectWorkspaceConfig {
    #[validate]
    pub inherited_tasks: ProjectWorkspaceInheritedTasksConfig,
}
