use moon_common::Id;
use rustc_hash::FxHashMap;
use std::path::Path;

pub struct ProjectExpanderContext<'graph> {
    /// Mapping of aliases to their project IDs.
    pub aliases: FxHashMap<&'graph str, &'graph Id>,

    /// Workspace root, of course.
    pub workspace_root: &'graph Path,
}
