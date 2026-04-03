use crate::project_builder::ProjectBuildData;
use moon_common::Id;
use moon_task::{Target, TaskOptions};
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct TaskBuildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip)]
    pub options: TaskOptions,
}

impl TaskBuildData {
    pub fn resolve_target(
        target: &Target,
        project_data: &FxHashMap<Id, ProjectBuildData>,
    ) -> miette::Result<Target> {
        // Target may be using an alias!
        let project_id = ProjectBuildData::resolve_id(target.get_project_id()?, project_data);

        // IDs should be valid here, so ignore the result
        Target::new_project(&project_id, &target.task_id)
    }
}
