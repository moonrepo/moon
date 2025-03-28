use super::common::clean_git_version;
use super::git_error::GitError;
use super::tree::*;
use crate::process_cache::ProcessCache;
use crate::touched_files::*;
use crate::vcs::Vcs;
use async_trait::async_trait;
use miette::IntoDiagnostic;
use moon_common::path::{RelativePathBuf, WorkspaceRelativePath, WorkspaceRelativePathBuf};
use moon_env_var::GlobalEnvBag;
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

#[derive(Debug)]
pub struct Gitx {
    /// Default branch name.
    pub default_branch: Arc<String>,

    /// List of remotes to use as merge candidates.
    pub remote_candidates: Vec<String>,

    /// Root of the repository that contains `.git`, any submodules,
    /// subtrees, and worktrees.
    pub repository_root: PathBuf,

    /// List of submodule trees within the repository.
    pub submodules: Vec<GitTree>,

    /// Root of the moon workspace. This may be nested within
    /// the repository root, or worktree root.
    pub workspace_root: PathBuf,

    /// Root of a worktree, as denoted by a `.git` file.
    /// Not defined if not in a worktree.
    pub worktree_root: Option<PathBuf>,

    /// The current working tree. Either a worktree checkout,
    /// or the root of the repository itself.
    pub worktree: GitTree,
}

impl Gitx {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
        remote_candidates: &[String],
    ) -> miette::Result<Gitx> {
        debug!("Using git as a version control system");

        let workspace_root = workspace_root.as_ref();
        let mut process = ProcessCache::new("git", workspace_root);

        debug!(
            starting_dir = ?workspace_root,
            "Attempting to find a .git directory or file"
        );

        // Find the repository root and work tree
        let mut current_dir = workspace_root;
        let mut worktree_root = None;
        let repository_root;

        loop {
            let git_check = current_dir.join(".git");

            if git_check.exists() {
                if git_check.is_file() {
                    debug!(
                        git = ?git_check,
                        "Found a .git file (submodule or worktree root)"
                    );

                    worktree_root = Some(current_dir.to_path_buf());
                    // Don't break and continue searching for the actual root
                } else {
                    debug!(
                        git = ?git_check,
                        "Found a .git directory (repository root)"
                    );

                    repository_root = current_dir.to_path_buf();
                    break;
                }
            }

            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => {
                    debug!("Unable to find .git, falling back to workspace root");

                    repository_root = workspace_root.to_path_buf();
                    break;
                }
            };
        }

        // Load the main worktree and submodule trees
        let worktree = match &worktree_root {
            Some(root) => GitTree::load_worktree(root)?,
            None => GitTree::load(&repository_root)?,
        };

        let submodules = GitTree::load_submodules(&repository_root, &worktree.work_dir)?;

        // Create the instance and load ignore files
        let mut git = Gitx {
            default_branch: Arc::new(default_branch.as_ref().to_owned()),
            remote_candidates: remote_candidates.to_owned(),
            repository_root,
            submodules,
            workspace_root: workspace_root.to_path_buf(),
            worktree_root,
            worktree,
        };

        process.worktree_root = git.worktree.work_dir.clone();
        let process = Arc::new(process);

        for tree in git.submodules.iter_mut() {
            tree.process = Some(Arc::clone(&process));
            tree.load_ignore()?;
        }

        git.worktree.process = Some(process);
        git.worktree.load_ignore()?;

        Ok(git)
    }

    fn get_all_trees(&self) -> Vec<GitTree> {
        let mut trees = vec![self.worktree.clone()];
        trees.extend(self.submodules.clone());
        trees
    }

    fn get_process(&self) -> &ProcessCache {
        self.worktree.get_process()
    }
}

#[async_trait]
impl Vcs for Gitx {
    async fn get_local_branch(&self) -> miette::Result<Arc<String>> {
        if self.is_version_supported(">=2.22.0").await? {
            return self
                .get_process()
                .run(["branch", "--show-current"], true)
                .await;
        }

        self.get_process()
            .run(["rev-parse", "--abbrev-ref", "HEAD"], true)
            .await
    }

    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.get_process().run(["rev-parse", "HEAD"], true).await
    }

    async fn get_default_branch(&self) -> miette::Result<Arc<String>> {
        Ok(self.default_branch.clone())
    }

    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.get_process()
            .run(["rev-parse", &self.default_branch], true)
            .await
    }

    // TODO
    #[instrument(skip_all)]
    async fn get_file_hashes(
        &self,
        // TODO change
        files: &[String], // Workspace relative
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut objects = vec![];
        let mut map = BTreeMap::new();

        for file in files {
            let abs_file = self.get_process().workspace_root.join(file);

            // File must exist or git fails
            if abs_file.exists()
                && abs_file.is_file()
                && (allow_ignored || !self.is_ignored(&abs_file))
            {
                // When moon is setup in a sub-folder and not the git root,
                // we need to prefix the paths because `--stdin-paths` assumes
                // the paths are from the git root and don't work correctly...
                // if let Some(prefix) = &self.root_prefix {
                //     objects.push(prefix.join(file).as_str().to_owned());
                // } else {
                objects.push(file.to_owned());
                // }
            }
        }

        if objects.is_empty() {
            return Ok(map);
        }

        // Sort for deterministic caching within the vcs layer
        objects.sort();

        let mut command = self
            .get_process()
            .create_command(["hash-object", "--stdin-paths"]);

        command.set_continuous_pipe(true);

        // hash-object requires new lines
        command.input(objects.iter().map(|obj| format!("{obj}\n")));

        let output = self.get_process().run_command(command, true).await?;

        for (i, hash) in output.split('\n').enumerate() {
            if !hash.is_empty() {
                map.insert(
                    // self.to_workspace_relative_path(&objects[i]),
                    RelativePathBuf::from(&objects[i]),
                    hash.to_owned(),
                );
            }
        }

        Ok(map)
    }

    #[instrument(skip(self))]
    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut paths = vec![];
        let mut set = JoinSet::new();
        let mut include_root = false;

        // At the root, so also include files from all submodules, so
        // that we have a full list of files available
        if dir == "." || dir == "" {
            include_root = true;

            for submodule in self.submodules.clone() {
                let target_dir = dir.to_owned();

                set.spawn(async move { submodule.exec_ls_files(&target_dir).await });
            }
        }
        // In a submodule, so only extract files from the target directory
        else if let Some(submodule) = self
            .submodules
            .iter()
            .find(|submodule| dir.starts_with(&submodule.path))
            .cloned()
        {
            let target_dir = dir.strip_prefix(&submodule.path).unwrap().to_owned();

            set.spawn(async move { submodule.exec_ls_files(&target_dir).await });
        }
        // In a directory in the root tree
        else {
            include_root = true;
        }

        if include_root {
            let root = self.worktree.clone();
            let target_dir = dir.to_owned();

            set.spawn(async move { root.exec_ls_files(&target_dir).await });
        }

        while let Some(result) = set.join_next().await {
            paths.extend(result.into_diagnostic()??)
        }

        map_absolute_to_workspace_relative_paths(paths, &self.workspace_root)
    }

    async fn get_hooks_dir(&self) -> miette::Result<PathBuf> {
        // Only use the hooks path if it's within the current repository
        let is_in_repo = |dir: &Path| dir.is_absolute() && dir.starts_with(&self.repository_root);

        if let Ok(output) = self
            .get_process()
            .run(["config", "--get", "core.hooksPath"], true)
            .await
        {
            let dir = PathBuf::from(output.as_str());

            if is_in_repo(&dir) {
                return Ok(dir);
            }
        }

        if let Some(dir) = GlobalEnvBag::instance().get("GIT_DIR") {
            let dir = PathBuf::from(dir).join("hooks");

            if is_in_repo(&dir) {
                return Ok(dir);
            }
        }

        // Worktrees do not support a hooks folder at `.git/worktrees/<name>/hooks`,
        // so we need to use the hooks folder at `.git/hooks` instead
        Ok(self.repository_root.join(".git/hooks"))
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        Ok(self.repository_root.clone())
    }

    async fn get_repository_slug(&self) -> miette::Result<Arc<String>> {
        use git_url_parse::GitUrl;

        for candidate in &self.remote_candidates {
            if let Ok(output) = self
                .get_process()
                .run_with_formatter(["remote", "get-url", candidate], true, |out| {
                    if let Ok(url) = GitUrl::parse(&out) {
                        url.fullname
                    } else {
                        out
                    }
                })
                .await
            {
                return Ok(output);
            }
        }

        Err(GitError::ExtractRepoSlugFailed.into())
    }

    async fn get_touched_files(&self) -> miette::Result<TouchedFiles> {
        let mut touched_files = TouchedFiles::default();
        let mut set = JoinSet::new();

        for tree in self.get_all_trees() {
            set.spawn(async move { tree.exec_status().await });
        }

        while let Some(result) = set.join_next().await {
            touched_files.merge(result.into_diagnostic()??);
        }

        touched_files.into_workspace_relative(&self.workspace_root)
    }

    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let revision = if self.is_default_branch(revision) {
            "HEAD"
        } else {
            revision
        };

        // If there's only 1 commit on the revision,
        // then the diff command will error. So let's
        // extract the commit count and handle accordingly.
        let output = self
            .get_process()
            .run(["rev-list", "--count", revision], true)
            .await?;

        let prev_revision = if output.as_str() == "0" || output.is_empty() {
            revision.to_owned()
        } else {
            format!("{revision}~1")
        };

        self.get_touched_files_between_revisions(&prev_revision, revision)
            .await
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let mut touched_files = TouchedFiles::default();

        // TODO: Revisit submodules
        // https://github.com/moonrepo/moon/issues/1734
        // for result in futures::future::try_join_all(self.modules.values().filter_map(|module| {
        //     if module.is_root() {
        //         Some(self.exec_diff(module, base_revision, revision))
        //     } else {
        //         None
        //     }
        // }))
        // .await?
        // {
        //     touched_files.merge(result);
        // }

        Ok(touched_files)
    }

    async fn get_version(&self) -> miette::Result<Version> {
        let version = self
            .get_process()
            .run_with_formatter(["--version"], true, clean_git_version)
            .await?;

        Ok(
            Version::parse(&version).map_err(|error| GitError::InvalidVersion {
                error: Box::new(error),
            })?,
        )
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        let default_branch = &self.default_branch;

        if default_branch.as_str() == branch {
            return true;
        }

        if default_branch.contains('/') {
            return default_branch.ends_with(&format!("/{branch}"));
        }

        false
    }

    fn is_enabled(&self) -> bool {
        self.worktree.git_dir.exists()
    }

    // TODO
    fn is_ignored(&self, file: &Path) -> bool {
        self.worktree.is_ignored(file)
    }

    async fn is_shallow_checkout(&self) -> miette::Result<bool> {
        let result = if self.is_version_supported(">=2.15.0").await? {
            let result = self
                .get_process()
                .run(["rev-parse", "--is-shallow-repository"], true)
                .await?;

            result.as_str() == "true"
        } else {
            let result = self
                .get_process()
                .run(["rev-parse", "--git-dir"], true)
                .await?;

            result.contains("shallow")
        };

        Ok(result)
    }
}
