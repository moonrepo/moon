use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use moon_config::{DependencyConfig, TaskConfig};
use rustc_hash::FxHashMap;
use starbase_events::Event;
use std::path::PathBuf;

// Extend the project graph with additional information.

#[derive(Debug)]
pub struct ExtendProjectGraphEvent {
    pub sources: FxHashMap<Id, WorkspaceRelativePathBuf>,
    pub workspace_root: PathBuf,

    // Mutable values
    pub extended_aliases: FxHashMap<String, Id>,
}

impl Event for ExtendProjectGraphEvent {
    type Value = ();
}

// Extend an individual project with implicit dependencies or inferred tasks.

#[derive(Debug)]
pub struct ExtendProjectEvent {
    pub project_id: Id,
    pub project_source: WorkspaceRelativePathBuf,
    pub workspace_root: PathBuf,

    // Mutable values
    pub extended_dependencies: Vec<DependencyConfig>,
    pub extended_tasks: FxHashMap<Id, TaskConfig>,
}

impl Event for ExtendProjectEvent {
    type Value = ();
}
