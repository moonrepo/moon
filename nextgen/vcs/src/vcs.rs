use crate::touched_files::TouchedFiles;
use crate::vcs_error::VcsError;
use async_trait::async_trait;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf};
use semver::{Version, VersionReq};
use std::collections::BTreeMap;

pub type VcsResult<T> = Result<T, VcsError>;

#[async_trait]
pub trait Vcs {
    /// Get the local checkout branch name.
    async fn get_local_branch(&self) -> VcsResult<&str>;

    /// Get the revision hash/number of the local branch's HEAD.
    async fn get_local_branch_revision(&self) -> VcsResult<&str>;

    /// Get the remote checkout default name. Typically master/main on git, and trunk on svn.
    async fn get_default_branch(&self) -> VcsResult<&str>;

    /// Get the revision hash/number of the default branch's HEAD.
    async fn get_default_branch_revision(&self) -> VcsResult<&str>;

    /// Get a map of hashes for the provided files. Files *must* be relative from
    /// the workspace root.
    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
        batch_size: u16,
    ) -> VcsResult<BTreeMap<WorkspaceRelativePathBuf, String>>;

    /// Get a list of all files in the provided directory, recursing through all sub-directories.
    /// Directory *must* be relative from the workspace root.
    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> VcsResult<Vec<WorkspaceRelativePathBuf>>;

    /// Return the repository slug ("moonrepo/moon") of the current checkout.
    async fn get_repository_slug(&self) -> VcsResult<&str>;

    /// Determine touched files from the local index / working tree.
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles>;

    /// Determine touched files between a revision and its self (-1 revision).
    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> VcsResult<TouchedFiles>;

    /// Determine touched files between 2 revisions.
    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> VcsResult<TouchedFiles>;

    /// Get the version of the current VCS binary
    async fn get_version(&self) -> VcsResult<Version>;

    /// Return true if the provided branch matches the default branch.
    fn is_default_branch(&self, branch: &str) -> bool;

    /// Return true if the repo is currently VCS enabled.
    fn is_enabled(&self) -> bool;

    /// Return true if the provided file path has been ignored.
    fn is_ignored(&self, file: &str) -> bool;

    /// Return true if the current binary version matches the provided requirement.
    async fn is_version_supported(&self, req: &str) -> VcsResult<bool> {
        let version = self.get_version().await?;

        Ok(VersionReq::parse(req).unwrap().matches(&version))
    }
}
