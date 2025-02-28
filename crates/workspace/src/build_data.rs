use moon_common::Id;
use moon_common::path::WorkspaceRelativePathBuf;
use moon_config::{
    DependencyConfig, ProjectConfig, ProjectsAliasesList, ProjectsSourcesList, TaskConfig,
};
use moon_task::{Target, TaskOptions};
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use starbase_events::Event;
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ProjectBuildData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    #[serde(skip)]
    pub config: Option<ProjectConfig>,

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
                if build_data
                    .alias
                    .as_ref()
                    .is_some_and(|alias| alias == id_or_alias)
                {
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
    ) -> Target {
        // Target may be using an alias!
        let project_id = ProjectBuildData::resolve_id(
            target
                .get_project_id()
                .expect("Target requires a fully-qualified project scope!"),
            project_data,
        );

        // IDs should be valid here, so ignore the result
        Target::new(&project_id, &target.task_id).expect("Failed to format target!")
    }
}

// Extend the project graph with additional information.

#[derive(Debug)]
pub struct ExtendProjectGraphEvent {
    pub sources: ProjectsSourcesList,
    pub workspace_root: PathBuf,
}

#[derive(Debug, Default)]
pub struct ExtendProjectGraphData {
    pub aliases: ProjectsAliasesList,
}

impl Event for ExtendProjectGraphEvent {
    type Data = ExtendProjectGraphData;
}

// Extend an individual project with implicit dependencies or inferred tasks.

#[derive(Debug)]
pub struct ExtendProjectEvent {
    pub project_id: Id,
    pub project_source: WorkspaceRelativePathBuf,
    pub workspace_root: PathBuf,
}

#[derive(Debug, Default)]
pub struct ExtendProjectData {
    pub dependencies: Vec<DependencyConfig>,
    pub tasks: FxHashMap<Id, TaskConfig>,
}

impl Event for ExtendProjectEvent {
    type Data = ExtendProjectData;
}
