use crate::touched_files::TouchedFiles;
use crate::vcs_error::VcsError;
use async_trait::async_trait;

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

    /// Return true if the provided branch matches the default branch.
    fn is_default_branch(&self, branch: &str) -> bool;

    /// Return true if the repo is currently VCS enabled.
    fn is_enabled(&self) -> bool;

    /// Return true if the provided file path has been ignored.
    fn is_ignored(&self, file: &str) -> bool;
}
