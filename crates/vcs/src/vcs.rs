use crate::touched_files::TouchedFiles;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf};
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[async_trait]
pub trait Vcs: Debug {
    /// Get the local checkout branch name.
    async fn get_local_branch(&self) -> miette::Result<Arc<String>>;

    /// Get the revision hash/number of the local branch's HEAD.
    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>>;

    /// Get the remote checkout default name. Typically master/main on git, and trunk on svn.
    async fn get_default_branch(&self) -> miette::Result<Arc<String>>;

    /// Get the revision hash/number of the default branch's HEAD.
    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>>;

    /// Get a map of hashes for the provided files. Files *must* be relative from
    /// the workspace root.
    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>>;

    /// Get a list of all files in the provided directory, recursing through all sub-directories.
    /// Directory *must* be relative from the workspace root.
    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>>;

    /// Return an absolute path to the hooks directory, when applicable.
    async fn get_hooks_dir(&self) -> miette::Result<PathBuf>;

    /// Return an absolute path to the repository root.
    async fn get_repository_root(&self) -> miette::Result<PathBuf>;

    /// Return the repository slug ("moonrepo/moon") of the current checkout.
    async fn get_repository_slug(&self) -> miette::Result<Arc<String>>;

    /// Determine touched files from the local index / working tree.
    async fn get_touched_files(&self) -> miette::Result<TouchedFiles>;

    /// Determine touched files between a revision and its self (-1 revision).
    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<TouchedFiles>;

    /// Determine touched files between 2 revisions.
    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<TouchedFiles>;

    /// Get the version of the current VCS binary
    async fn get_version(&self) -> miette::Result<Version>;

    /// Return true if the provided branch matches the default branch.
    fn is_default_branch(&self, branch: &str) -> bool;

    /// Return true if the repo is currently VCS enabled.
    fn is_enabled(&self) -> bool;

    /// Return true if the provided file path has been ignored.
    fn is_ignored(&self, file: &Path) -> bool;

    /// Return true if the current repository is a shallow checkout.
    async fn is_shallow_checkout(&self) -> miette::Result<bool>;

    /// Return true if the current binary version matches the provided requirement.
    async fn is_version_supported(&self, req: &str) -> miette::Result<bool> {
        let version = self.get_version().await?;

        Ok(VersionReq::parse(req).into_diagnostic()?.matches(&version))
    }
}
