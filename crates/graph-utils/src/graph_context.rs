use moon_vcs::BoxedVcs;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Default)]
pub struct GraphContext {
    /// The VCS instance.
    pub vcs: Option<Arc<BoxedVcs>>,

    /// The current working directory.
    pub working_dir: PathBuf,

    /// The workspace root.
    pub workspace_root: PathBuf,
}
