use super::common::clean_git_version;
use super::git_error::GitError;
use super::tree::*;
use crate::changed_files::*;
use crate::process_cache::ProcessCache;
use crate::vcs::{Vcs, VcsHookEnvironment};
use async_trait::async_trait;
use git_url_parse::types::provider::GenericProvider;
use gix::{
    discover::Error as RepoError, discover::upwards::Error as UpwardsError, repository::Kind,
};
use miette::IntoDiagnostic;
use moon_common::path::{
    PathExt, RelativePathBuf, WorkspaceRelativePath, WorkspaceRelativePathBuf, clean_components,
};
use moon_process::find_command_on_path;
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::debug;

fn get_repository_root(common_dir: &Path) -> PathBuf {
    // Worktrees trail with "../.." so we need to remove them
    let dir = clean_components(common_dir);

    // We also want to repository root, not the `.git` directory,
    // so remove it and return the parent
    if dir.ends_with(".git") {
        return dir.parent().unwrap().into();
    }

    dir
}

enum GitConfigAction {
    Get(String),
    Set(String, String),
    Unset(String),
}

#[derive(Debug)]
pub struct Git {
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

    /// The current working tree. Either a worktree checkout,
    /// or the root of the repository itself.
    pub worktree: GitTree,
}

impl Git {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
        remote_candidates: &[String],
    ) -> miette::Result<Git> {
        debug!("Using git as a version control system (using v2 implementation)");

        let workspace_root = workspace_root.as_ref();

        let mut process = ProcessCache::new("git", workspace_root);
        process.env.insert("GIT_OPTIONAL_LOCKS".into(), "0".into());
        process.env.insert("GIT_PAGER".into(), "".into());

        debug!(
            starting_dir = ?workspace_root,
            "Attempting to find a .git directory or file"
        );

        // Find the repository root and work tree
        let mut worktree = GitTree::default();
        let mut submodules = vec![];
        let mut repository_root = workspace_root.to_path_buf();

        match gix::discover(workspace_root) {
            Ok(repo) => {
                repository_root = get_repository_root(repo.common_dir());

                worktree.work_dir = clean_components(repo.workdir().unwrap_or(repo.git_dir()));
                worktree.git_dir = clean_components(repo.git_dir());

                match repo.kind() {
                    Kind::Bare => {
                        debug!(
                            git = ?worktree.work_dir,
                            "Found a bare repository (things may not work correctly)"
                        );
                    }
                    Kind::Submodule => {
                        debug!(
                            git = ?worktree.work_dir,
                            "Found a .git file (submodule root)"
                        );

                        worktree.type_of = GitTreeType::Submodule;
                    }
                    Kind::WorkTree { is_linked } => {
                        if is_linked {
                            debug!(
                                git = ?worktree.work_dir,
                                "Found a .git file (worktree root)"
                            );

                            worktree.type_of = GitTreeType::Worktree;
                        } else {
                            debug!(
                                git = ?worktree.work_dir,
                                "Found a .git directory (repository root)"
                            );
                        };
                    }
                };

                if let Some(subs) =
                    repo.submodules()
                        .map_err(|error| GitError::SubmodulesLoadFailed {
                            error: Box::new(error),
                        })?
                {
                    debug!(
                        modules_file = ?worktree.work_dir.join(".gitmodules"),
                        "Loading submodules from .gitmodules",
                    );

                    for sub in subs {
                        if let Ok(work_dir) = sub.work_dir()
                            && let Ok(rel_path) = sub.path()
                        {
                            submodules.push(GitTree {
                                work_dir: clean_components(if work_dir.is_absolute() {
                                    work_dir
                                } else {
                                    repository_root.join(work_dir)
                                }),
                                git_dir: clean_components(sub.git_dir()),
                                type_of: GitTreeType::Submodule,
                                path: RelativePathBuf::from(rel_path.to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            Err(error) => {
                if let RepoError::Discover(inner) = &error
                    && matches!(
                        inner,
                        UpwardsError::NoGitRepository { .. }
                            | UpwardsError::NoGitRepositoryWithinCeiling { .. }
                            | UpwardsError::NoGitRepositoryWithinFs { .. }
                    )
                {
                    debug!("Unable to find .git, falling back to workspace root");

                    worktree.git_dir = workspace_root.join(".git");
                    worktree.work_dir = workspace_root.into();
                } else {
                    return Err(GitError::RepositoryLoadFailed {
                        error: Box::new(error),
                    }
                    .into());
                }
            }
        };

        // Create the instance and load ignore files
        let mut git = Git {
            default_branch: Arc::new(default_branch.as_ref().to_owned()),
            remote_candidates: remote_candidates.to_owned(),
            repository_root,
            submodules,
            workspace_root: workspace_root.to_path_buf(),
            worktree,
        };

        let process = Arc::new(process);

        for tree in &mut git.submodules {
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

    pub async fn get_remote_default_branch(&self) -> miette::Result<Arc<String>> {
        let extract_branch = |result: Arc<String>| -> Option<Arc<String>> {
            if let Some(branch) = result.strip_prefix("origin/") {
                return Some(Arc::new(branch.to_owned()));
            } else if let Some(branch) = result.strip_prefix("upstream/") {
                return Some(Arc::new(branch.to_owned()));
            }

            None
        };

        if let Ok(result) = self
            .get_process()
            .run(["rev-parse", "--abbrev-ref", "origin/HEAD"], true)
            .await
            && let Some(branch) = extract_branch(result)
        {
            return Ok(branch);
        };

        if let Ok(result) = self
            .get_process()
            .run(
                ["symbolic-ref", "refs/remotes/origin/HEAD", "--short"],
                true,
            )
            .await
            && let Some(branch) = extract_branch(result)
        {
            return Ok(branch);
        };

        Ok(self.default_branch.clone())
    }

    async fn exec_config(
        &self,
        action: GitConfigAction,
        other_args: Vec<String>,
    ) -> miette::Result<Arc<String>> {
        let use_new_commands = self.is_version_supported(">=2.46.0").await?;
        let other_args = other_args.iter().map(|arg| arg.as_str());
        let mut args = vec!["config"];

        match &action {
            GitConfigAction::Get(key) => {
                if use_new_commands {
                    args.push("get");
                } else {
                    args.push("--get");
                }

                args.extend(other_args);
                args.push(key);
            }
            GitConfigAction::Set(key, value) => {
                if use_new_commands {
                    args.push("set");
                }

                args.extend(other_args);
                args.push(key);
                args.push(value);
            }
            GitConfigAction::Unset(key) => {
                if use_new_commands {
                    args.push("unset");
                } else {
                    args.push("--unset");
                }

                args.extend(other_args);
                args.push(key);
            }
        };

        self.get_process().run(args, true).await
    }
}

#[async_trait]
impl Vcs for Git {
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

    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut objects = vec![];
        let mut map = BTreeMap::new();
        let work_dir = &self.worktree.work_dir;

        for file in files {
            let abs_file = file.to_logical_path(&self.workspace_root);

            // File must exist and must not be a directory or Git fails
            if abs_file.exists()
                && abs_file.is_file()
                && (allow_ignored || !self.is_ignored(&abs_file))
            {
                // When moon is setup in a sub-folder and not the Git root,
                // we need to prefix the paths because `--stdin-paths` assumes
                // the paths are from the worktree root and don't work correctly...
                if &self.workspace_root == work_dir {
                    objects.push(file.to_string());
                } else {
                    objects.push(
                        abs_file
                            .relative_to(work_dir)
                            .into_diagnostic()?
                            .to_string(),
                    );
                }
            }
        }

        if objects.is_empty() {
            return Ok(map);
        }

        // Sort for deterministic caching within the vcs layer
        objects.sort();

        let process = self.get_process();
        let mut command = process.create_command_in_cwd(["hash-object", "--stdin-paths"], work_dir);

        command.set_continuous_pipe(true);

        // hash-object requires new lines
        command.input(objects.iter().map(|obj| format!("{obj}\n")));

        let output = process.run_command(command, true).await?;

        for (i, hash) in output.split('\n').enumerate() {
            if !hash.is_empty() {
                map.insert(
                    work_dir
                        .join(&objects[i])
                        .relative_to(&self.workspace_root)
                        .into_diagnostic()?,
                    hash.to_owned(),
                );
            }
        }

        Ok(map)
    }

    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut paths = vec![];

        // Use an absolute path t avoid issues where moon is nested
        // within the repository and not at the root
        let abs_dir = dir.to_logical_path(&self.workspace_root);

        // In a submodule, so only extract files from the target directory
        if let Some(submodule) = self
            .submodules
            .iter()
            .find(|submodule| abs_dir.starts_with(&submodule.work_dir))
        {
            let target_dir = abs_dir.relative_to(&submodule.work_dir).into_diagnostic()?;

            paths.extend(submodule.exec_ls_files(&target_dir).await?);
        }
        // In a directory in the root tree
        else {
            let target_dir = abs_dir
                .relative_to(&self.worktree.work_dir)
                .into_diagnostic()?;

            // At the root, so also include files from all submodules, so
            // that we have a full list of files available
            if dir == "." || dir == "" {
                let mut set = JoinSet::new();

                for tree in self.get_all_trees() {
                    let target_dir = target_dir.clone();

                    set.spawn(async move { tree.exec_ls_files(&target_dir).await });
                }

                while let Some(result) = set.join_next().await {
                    paths.extend(result.into_diagnostic()??)
                }
            } else {
                paths.extend(self.worktree.exec_ls_files(&target_dir).await?);
            }
        }

        let mut new_paths = vec![];

        for path in paths {
            new_paths.push(path.relative_to(&self.workspace_root).into_diagnostic()?);
        }

        Ok(new_paths)
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
                    if let Ok(url) =
                        GitUrl::parse(&out).and_then(|url| url.provider_info::<GenericProvider>())
                    {
                        url.fullname()
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

    async fn get_changed_files(&self) -> miette::Result<ChangedFiles> {
        let mut changed_files = ChangedFiles::default();
        let mut set = JoinSet::new();

        for tree in self.get_all_trees() {
            set.spawn(async move { tree.exec_status().await });
        }

        while let Some(result) = set.join_next().await {
            changed_files.merge(result.into_diagnostic()??);
        }

        changed_files.into_workspace_relative(&self.workspace_root)
    }

    async fn get_changed_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<ChangedFiles> {
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

        self.get_changed_files_between_revisions(&prev_revision, revision)
            .await
    }

    async fn get_changed_files_between_revisions(
        &self,
        base_revision: &str,
        head_revision: &str, // Can be empty
    ) -> miette::Result<ChangedFiles> {
        let mut changed_files = ChangedFiles::default();

        // Determine the merge base revision based on the base/head
        let merge_base = self
            .worktree
            .exec_merge_base(base_revision, head_revision, &self.remote_candidates)
            .await?;
        let merge_base_revision = merge_base
            .as_ref()
            .map(|rev| rev.as_str())
            .unwrap_or(base_revision);

        // Load from root repo
        changed_files.merge(self.worktree.exec_diff(merge_base_revision, "").await?);

        // Load from each submodule
        if !self.submodules.is_empty() {
            let mut set = JoinSet::new();

            // Since submodules are separate repos with their own history,
            // we need to extract the base/head revisions from their history,
            // using the changes in the current repo
            let mut base_tree = self.worktree.exec_ls_tree(merge_base_revision).await?;
            let mut head_tree = self
                .worktree
                .exec_ls_tree(if head_revision.is_empty() {
                    "HEAD"
                } else {
                    head_revision
                })
                .await?;

            for submodule in &self.submodules {
                if let Some(base) = base_tree.remove(&submodule.work_dir) {
                    let head = head_tree.remove(&submodule.work_dir).unwrap_or_default();

                    if base != head {
                        let submodule = submodule.to_owned();
                        set.spawn(async move { submodule.exec_diff(&base, &head).await });
                    }
                }
            }

            if !set.is_empty() {
                while let Some(result) = set.join_next().await {
                    changed_files.merge(result.into_diagnostic()??);
                }
            }
        }

        changed_files.into_workspace_relative(&self.workspace_root)
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

    async fn get_working_root(&self) -> miette::Result<PathBuf> {
        Ok(self.worktree.work_dir.clone())
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
        self.worktree.git_dir.exists() && find_command_on_path("git".into()).is_some()
    }

    fn is_ignored(&self, file: &Path) -> bool {
        // Check if this path is within a submodule,
        // and if so, use the ignore list there
        for submodule in &self.submodules {
            if file.starts_with(&submodule.work_dir) {
                return submodule.is_ignored(file);
            }
        }

        // Otherwise it's within the current worktree
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

    async fn setup_hooks(&self) -> miette::Result<Option<VcsHookEnvironment>> {
        let mut env = VcsHookEnvironment {
            // The working directory is the worktree root, not the workspace root
            working_dir: self.worktree.work_dir.clone(),
            ..Default::default()
        };

        // Check if the path has already been configured
        if let Ok(output) = self
            .exec_config(
                GitConfigAction::Get("core.hooksPath".into()),
                vec!["--worktree".into()],
            )
            .await
        {
            let dir = PathBuf::from(output.as_str());

            if !output.is_empty() && dir.starts_with(&self.workspace_root) {
                env.hooks_dir = dir;

                return Ok(Some(env));
            }
        }

        // Enable config support for worktrees
        if self.worktree.is_worktree() {
            self.exec_config(
                GitConfigAction::Set("extensions.worktreeConfig".into(), "true".into()),
                vec![],
            )
            .await?;
        }

        // Otherwise update the config with the path
        let hooks_dir = self.workspace_root.join(".moon").join("hooks");

        self.exec_config(
            GitConfigAction::Set(
                "core.hooksPath".into(),
                hooks_dir.to_string_lossy().to_string(),
            ),
            vec!["--worktree".into()],
        )
        .await?;

        env.hooks_dir = hooks_dir;

        Ok(Some(env))
    }

    async fn teardown_hooks(&self) -> miette::Result<()> {
        self.exec_config(
            GitConfigAction::Unset("core.hooksPath".into()),
            vec!["--worktree".into()],
        )
        .await?;

        Ok(())
    }
}
