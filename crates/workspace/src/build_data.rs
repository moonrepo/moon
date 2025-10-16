use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::ProjectConfig;
use moon_pdk_api::ExtendProjectOutput;
use moon_task::{Target, TaskOptions};
use petgraph::graph::NodeIndex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ProjectBuildData {
    #[serde(skip_serializing_if = "FxHashSet::is_empty")]
    pub aliases: FxHashSet<String>,

    #[serde(skip)]
    pub config: Option<ProjectConfig>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<ExtendProjectOutput>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_index: Option<NodeIndex>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_id: Option<Id>,

    pub source: WorkspaceRelativePathBuf,
}

impl ProjectBuildData {
    pub fn resolve_id(id_or_alias: &str, project_data: &FxHashMap<Id, ProjectBuildData>) -> Id {
        if project_data.contains_key(id_or_alias) {
            Id::raw(id_or_alias)
        } else {
            match project_data.iter().find_map(|(id, build_data)| {
                if build_data.aliases.contains(id_or_alias) {
                    Some(id)
                } else {
                    None
                }
            }) {
                Some(project_id) => project_id.to_owned(),
                None => Id::raw(id_or_alias),
            }
        }
    }
}

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
        Target::new(&project_id, &target.task_id)
    }
}
