use crate::changed_files::{ChangedFiles, ChangedStatus};
use crate::git::Git;
use crate::process_cache::ProcessCache;
use crate::vcs::{Vcs, VcsHookEnvironment};
use async_trait::async_trait;
use moon_common::path::{WorkspaceRelativePath, WorkspaceRelativePathBuf};
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};
use tracing::{debug, warn};

static HOOK_WARNING: Once = Once::new();

/// A jj-aware wrapper around the [`Git`] backend, active inside any Jujutsu
/// workspace colocated-at-root, or in a workspace.
///
/// jj keeps Git's object store and working tree in sync, so content-addressed
/// operations (file hashing, the file tree, repo discovery) still delegate to
/// the Git backend. But the working-copy and revision operations are routed
/// through `jj`.
#[derive(Debug)]
pub struct JjAwareGit {
    git: Git,
    jj: ProcessCache,
}

impl JjAwareGit {
    /// Returns `true` if `workspace_root` is inside a Jujutsu workspace —
    /// colocated-at-root *or* a secondary workspace. Uses `jj root` rather than
    /// probing for `.jj`, so detection is correct regardless of where `.jj` and
    /// `.git` sit (in a secondary workspace they diverge), and it doubles as a
    /// "is the `jj` binary available" gate.
    pub fn is_jj_workspace(git: &Git) -> bool {
        std::process::Command::new("jj")
            .arg("root")
            .current_dir(&git.workspace_root)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    pub fn new(git: Git) -> Self {
        debug!("Jujutsu workspace detected, enabling jj-aware VCS behavior");

        let jj = ProcessCache::new("jj", &git.workspace_root);

        Self { git, jj }
    }

    /// Run a `jj diff … --summary` and parse it into [`ChangedFiles`]. Because
    /// `jj` is scoped to the workspace by cwd, this reflects *this workspace's*
    /// `@` — unlike the Git backend, which in a secondary workspace sees the
    /// parent repo and reports nothing.
    async fn jj_changed(&self, args: &[&str]) -> miette::Result<ChangedFiles> {
        let output = self.jj.run(args.iter().copied(), true).await?;

        Ok(parse_changed_files(&output))
    }

    /// Resolve a human-meaningful label for the `@` change: a bookmark name if
    /// one points at `@`, otherwise the short change-id. Returns `None` if `jj`
    /// can't be queried (so callers can fall back to the Git answer).
    async fn resolve_working_change(&self) -> Option<String> {
        // Bookmarks pointing at the working-copy change, if any.
        if let Ok(bookmarks) = self
            .jj
            .run(
                [
                    "log",
                    "--no-graph",
                    "--color",
                    "never",
                    "-r",
                    "@",
                    "-T",
                    "bookmarks",
                ],
                true,
            )
            .await
        {
            let bookmarks = bookmarks.trim();
            if let Some(first) = bookmarks.split_whitespace().next() {
                return Some(first.to_owned());
            }
        }

        // Otherwise the stable change-id (jj's identity for the working copy).
        if let Ok(change_id) = self
            .jj
            .run(
                [
                    "log",
                    "--no-graph",
                    "--color",
                    "never",
                    "-r",
                    "@",
                    "-T",
                    "change_id.short(8)",
                ],
                true,
            )
            .await
        {
            let change_id = change_id.trim();
            if !change_id.is_empty() {
                return Some(change_id.to_owned());
            }
        }

        None
    }
}

#[async_trait]
impl Vcs for JjAwareGit {
    // --- jj-aware overrides -------------------------------------------------

    async fn get_local_branch(&self) -> miette::Result<Arc<String>> {
        if let Some(label) = self.resolve_working_change().await {
            return Ok(Arc::new(label));
        }

        // jj unavailable/unexpected output: fall back to Git's (likely empty) answer.
        self.git.get_local_branch().await
    }

    async fn setup_hooks(&self) -> miette::Result<Option<VcsHookEnvironment>> {
        HOOK_WARNING.call_once(|| {
            warn!(
                "Jujutsu checkout detected: moon's git hooks won't run on `jj` commits or \
                 operations (jj has no hook system). They remain active for plain-git contributors."
            );
        });

        self.git.setup_hooks().await
    }

    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>> {
        // This workspace's `@` commit. (The Git backend would report the parent
        // repo's HEAD in a secondary workspace.)
        if let Ok(commit) = self
            .jj
            .run(
                [
                    "log",
                    "--no-graph",
                    "--color",
                    "never",
                    "-r",
                    "@",
                    "-T",
                    "commit_id",
                ],
                true,
            )
            .await
        {
            let commit = commit.trim();
            if !commit.is_empty() {
                return Ok(Arc::new(commit.to_owned()));
            }
        }

        self.git.get_local_branch_revision().await
    }

    async fn get_changed_files(&self) -> miette::Result<ChangedFiles> {
        // Working-copy changes = the `@` change's diff vs its parent. Routed
        // through jj so it's correct in a secondary workspace (where git sees
        // the parent repo and reports nothing).
        self.jj_changed(&["diff", "-r", "@", "--summary", "--color", "never"])
            .await
    }

    async fn get_changed_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<ChangedFiles> {
        // Files changed *in* `revision` (vs its parent).
        self.jj_changed(&[
            "diff",
            "-r",
            translate_rev(revision),
            "--summary",
            "--color",
            "never",
        ])
        .await
    }

    async fn get_changed_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<ChangedFiles> {
        self.jj_changed(&[
            "diff",
            "--from",
            translate_rev(base_revision),
            "--to",
            translate_rev(revision),
            "--summary",
            "--color",
            "never",
        ])
        .await
    }

    // --- delegated to the Git backend --------------------------------------

    async fn get_default_branch(&self) -> miette::Result<Arc<String>> {
        self.git.get_default_branch().await
    }

    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.git.get_default_branch_revision().await
    }

    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        self.git.get_file_hashes(files, allow_ignored).await
    }

    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        self.git.get_file_tree(dir).await
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        self.git.get_repository_root().await
    }

    async fn get_repository_slug(&self) -> miette::Result<Arc<String>> {
        self.git.get_repository_slug().await
    }

    async fn get_version(&self) -> miette::Result<Version> {
        self.git.get_version().await
    }

    async fn get_working_root(&self) -> miette::Result<PathBuf> {
        self.git.get_working_root().await
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        self.git.is_default_branch(branch)
    }

    fn is_enabled(&self) -> bool {
        self.git.is_enabled()
    }

    fn is_ignored(&self, file: &Path) -> bool {
        self.git.is_ignored(file)
    }

    async fn is_shallow_checkout(&self) -> miette::Result<bool> {
        self.git.is_shallow_checkout().await
    }

    async fn teardown_hooks(&self) -> miette::Result<()> {
        self.git.teardown_hooks().await
    }
}

/// Translate the git-ish revisions moon passes into jj equivalents. An empty
/// head means "the current working tree" per the `Vcs` trait contract, and
/// git's `HEAD` is jj's `@`; both map to `@`. Concrete commit ids and bookmark
/// names pass through (jj accepts both).
fn translate_rev(rev: &str) -> &str {
    if rev.is_empty() || rev == "HEAD" {
        "@"
    } else {
        rev
    }
}

/// Parse `jj diff --summary` output — `"<status> <path>"` lines — into
/// [`ChangedFiles`]. Paths are workspace-relative (jj emits them relative to
/// the workspace root). jj has no "untracked" state (the working copy is
/// auto-snapshotted into `@`), so new files surface as `Added`.
fn parse_changed_files(output: &str) -> ChangedFiles {
    let mut changed = ChangedFiles::default();

    for line in output.lines() {
        let Some((status, path)) = line.trim_end().split_once(' ') else {
            continue;
        };

        let status = match status {
            "A" => ChangedStatus::Added,
            "D" => ChangedStatus::Deleted,
            // M, plus R (rename) / C (copy) keyed on their destination.
            _ => ChangedStatus::Modified,
        };

        // Renames/copies render as "old => new"; key on the destination path.
        let path = path.rsplit(" => ").next().unwrap_or(path).trim();

        changed
            .files
            .entry(WorkspaceRelativePathBuf::from(path))
            .or_default()
            .push(status);
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    // Local/manual demo: must run inside a colocated jj checkout (e.g. this repo).
    // Plain `cargo test` skips it so CI/non-jj checkouts stay green.
    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "requires a colocated jj checkout; run with `-- --ignored --nocapture`"]
    async fn reports_working_change_on_colocated_repo() {
        let cwd = std::env::current_dir().unwrap();
        let git = Git::load(&cwd, "master", &["origin".to_string()]).unwrap();

        assert!(JjAwareGit::is_jj_workspace(&git), "expected a jj workspace");

        let plain_git_branch = git.get_local_branch().await.unwrap();
        let vcs = JjAwareGit::new(git);
        let jj_branch = vcs.get_local_branch().await.unwrap();

        println!("plain git get_local_branch()  = {plain_git_branch:?}");
        println!("jj-aware get_local_branch()    = {jj_branch:?}");

        assert!(
            !jj_branch.is_empty(),
            "expected a jj bookmark/change-id, not an empty branch"
        );
    }

    #[test]
    fn parses_jj_diff_summary() {
        let changed =
            parse_changed_files("A apps/web/new.rb\nM lib/util.rb\nD old.rb\nR a.rb => b.rb\n");

        assert_eq!(
            changed
                .files
                .get(&WorkspaceRelativePathBuf::from("apps/web/new.rb")),
            Some(&vec![ChangedStatus::Added])
        );
        assert_eq!(
            changed
                .files
                .get(&WorkspaceRelativePathBuf::from("lib/util.rb")),
            Some(&vec![ChangedStatus::Modified])
        );
        assert_eq!(
            changed.files.get(&WorkspaceRelativePathBuf::from("old.rb")),
            Some(&vec![ChangedStatus::Deleted])
        );
        // Rename keyed on the destination path.
        assert!(
            changed
                .files
                .contains_key(&WorkspaceRelativePathBuf::from("b.rb"))
        );
    }
}
