use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependenciesWorkspaceRole {
    // Workspace
    WorkspaceRoot,
    WorkspaceMember,
    // Non-workspace
    PackageRoot,
}

#[derive(Debug)]
pub struct DependenciesWorkspace {
    pub root: PathBuf,
    // If not a multi-package workspace, will be `None`.
    // Otherwise will be a list of packages in the workspace.
    pub members: Option<Vec<String>>,
}
