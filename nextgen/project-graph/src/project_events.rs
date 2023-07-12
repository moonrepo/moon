use std::path::PathBuf;

use moon_common::path::WorkspaceRelativePathBuf;
use moon_common::Id;
use rustc_hash::FxHashMap;
use starbase_events::Event;

pub struct ExtendProjectGraph {
    pub aliases: FxHashMap<String, Id>,
}

#[derive(Debug)]
pub struct ExtendProjectGraphEvent {
    pub sources: FxHashMap<Id, WorkspaceRelativePathBuf>,
    pub workspace_root: PathBuf,
}

impl Event for ExtendProjectGraphEvent {
    type Value = ExtendProjectGraph;
}
