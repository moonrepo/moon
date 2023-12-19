use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{DependencyConfig, ProjectsSourcesList, TaskConfig};
use rustc_hash::FxHashMap;
use starbase_events::Event;
use std::path::PathBuf;

// Extend the project graph with additional information.

#[derive(Debug)]
pub struct ExtendProjectGraphEvent {
    pub sources: ProjectsSourcesList,
    pub workspace_root: PathBuf,
}

#[derive(Debug, Default)]
pub struct ExtendProjectGraphData {
    pub aliases: FxHashMap<String, Id>,
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
