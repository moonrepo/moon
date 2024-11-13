use std::{path::PathBuf, sync::Arc};

#[derive(Clone, Default)]
pub struct GraphExpanderContext {
    /// The current VCS branch.
    pub vcs_branch: Arc<String>,

    /// The VCS repository slug.
    pub vcs_repository: Arc<String>,

    /// The current VCS revision, commit, etc.
    pub vcs_revision: Arc<String>,

    /// The current working directory.
    pub working_dir: PathBuf,

    /// The workspace root.
    pub workspace_root: PathBuf,
}
