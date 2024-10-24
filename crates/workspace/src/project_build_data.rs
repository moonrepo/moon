use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{
    DependencyConfig, ProjectConfig, ProjectsAliasesList, ProjectsSourcesList, TaskConfig,
};
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
    pub fn from_source(source: &str) -> Self {
        Self {
            source: source.into(),
            ..Default::default()
        }
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
