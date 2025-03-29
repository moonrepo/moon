use crate::git_submodule::*;
use crate::git_worktree::*;
use crate::gitx::common::*;
use crate::process_cache::ProcessCache;
use crate::touched_files::TouchedFiles;
use crate::vcs::Vcs;
use async_trait::async_trait;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::Diagnostic;
use moon_common::path::{RelativePath, RelativePathBuf, WorkspaceRelativePathBuf};
use moon_common::{Style, Stylize};
use moon_env_var::GlobalEnvBag;
use rustc_hash::FxHashSet;
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, instrument};

#[derive(Error, Debug, Diagnostic)]
pub enum GitError {
    #[diagnostic(code(git::invalid_version))]
    #[error("Invalid or unsupported git version.")]
    InvalidVersion {
        #[source]
        error: Box<semver::Error>,
    },

    #[diagnostic(code(git::ignore::load_invalid))]
    #[error("Failed to load and parse {}.", ".gitignore".style(Style::File))]
    GitignoreLoadFailed {
        #[source]
        error: Box<ignore::Error>,
    },

    #[diagnostic(code(git::repository::extract_slug))]
    #[error("Failed to extract a repository slug from git remote candidates.")]
    ExtractRepoSlugFailed,

    #[diagnostic(code(git::worktree::parse_failed))]
    #[error("Failed to parse .git worktree file.")]
    ParseWorktreeFailed,

    #[diagnostic(code(git::worktree::load_failed))]
    #[error("Failed to load .git worktree file {}.", .path.style(Style::Path))]
    LoadWorktreeFailed {
        path: PathBuf,
        #[source]
        error: Box<std::io::Error>,
    },
}

#[derive(Debug)]
pub struct Git {
    /// Ignore rules derived from a root `.gitignore` file.
    ignore: Option<Gitignore>,

    /// Default git branch name.
    pub default_branch: Arc<String>,

    /// Root of the `.git` directory.
    pub git_root: PathBuf,

    /// Run and cache `git` commands.
    pub process: ProcessCache,

    /// List of remotes to use as merge candidates.
    pub remote_candidates: Vec<String>,

    /// Root of the repository that contains `.git`.
    pub repository_root: PathBuf,

    /// Path between the git and workspace root.
    pub root_prefix: Option<RelativePathBuf>,

    /// If in a git worktree, information about it's location (the `.git` file).
    pub worktree: Option<GitWorktree>,

    /// Map of submodules within the repository.
    /// The root is also considered a module to keep things easy.
    modules: BTreeMap<String, GitModule>,
}

impl Git {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
        remote_candidates: &[String],
    ) -> miette::Result<Git> {
        debug!("Using git as a version control system");

        let workspace_root = workspace_root.as_ref();
        let default_branch = default_branch.as_ref();

        debug!(
            starting_dir = ?workspace_root,
            "Attempting to find a .git directory or file"
        );

        // Find the .git dir
        let mut current_dir = workspace_root;
        let mut worktree = None;
        let repository_root;
        let git_root;

        loop {
            let git_check = current_dir.join(".git");

            if git_check.exists() {
                if git_check.is_file() {
                    debug!(
                        git = ?git_check,
                        "Found a .git file (submodule or worktree root)"
                    );

                    worktree = Some(GitWorktree {
                        checkout_dir: current_dir.to_path_buf(),
                        git_dir: extract_gitdir_from_worktree(&git_check)?,
                    });

                    // Don't break and continue searching for the root
                } else {
                    debug!(
                        git = ?git_check,
                        "Found a .git directory (repository root)"
                    );

                    git_root = git_check.to_path_buf();
                    repository_root = current_dir.to_path_buf();
                    break;
                }
            }

            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => {
                    debug!("Unable to find .git, falling back to workspace root");

                    git_root = workspace_root.join(".git");
                    repository_root = workspace_root.to_path_buf();
                    break;
                }
            };
        }

        // Load .gitignore
        let ignore_path = repository_root.join(".gitignore");
        let mut ignore: Option<Gitignore> = None;

        if ignore_path.exists() {
            debug!(
                ignore_file = ?ignore_path,
                "Loading ignore rules from .gitignore",
            );

            let mut builder = GitignoreBuilder::new(&repository_root);

            if let Some(error) = builder.add(ignore_path) {
                return Err(GitError::GitignoreLoadFailed {
                    error: Box::new(error),
                }
                .into());
            }

            ignore = Some(
                builder
                    .build()
                    .map_err(|error| GitError::GitignoreLoadFailed {
                        error: Box::new(error),
                    })?,
            );
        }

        // Load .gitmodules
        let modules_path = repository_root.join(".gitmodules");
        let mut modules = BTreeMap::from_iter([(
            "(root)".into(),
            GitModule {
                checkout_dir: repository_root.clone(),
                git_dir: git_root.clone(),
                ..Default::default()
            },
        )]);

        if modules_path.exists() {
            debug!(
                modules_file = ?modules_path,
                "Loading submodules from .gitmodules",
            );

            modules.extend(parse_gitmodules_file(&modules_path, &repository_root)?);
        }

        let git = Git {
            default_branch: Arc::new(default_branch.to_owned()),
            ignore,
            remote_candidates: remote_candidates.to_owned(),
            root_prefix: if repository_root == workspace_root {
                None
            } else if let Some(tree) = &worktree {
                if tree.checkout_dir == workspace_root {
                    None
                } else {
                    RelativePathBuf::from_path(
                        workspace_root.strip_prefix(&tree.checkout_dir).unwrap(),
                    )
                    .ok()
                }
            } else {
                RelativePathBuf::from_path(workspace_root.strip_prefix(&repository_root).unwrap())
                    .ok()
            },
            repository_root,
            process: ProcessCache::new("git", workspace_root),
            git_root,
            worktree,
            modules,
        };

        Ok(git)
    }

    async fn get_merge_base(&self, base: &str, head: &str) -> miette::Result<Option<Arc<String>>> {
        let mut args = vec!["merge-base", head];
        let mut candidates = vec![base.to_owned()];

        for remote in &self.remote_candidates {
            candidates.push(format!("{remote}/{base}"));
        }

        // To start, we need to find a working base
        for candidate in &candidates {
            if self
                .process
                .run(["merge-base", candidate, head], true)
                .await
                .is_ok()
            {
                args.push(candidate);
            }
        }

        // Then we need to run it again and extract the base hash.
        // This is necessary to support comparisons between forks!
        if let Ok(hash) = self.process.run(args, true).await {
            return Ok(Some(hash));
        }

        Ok(None)
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
            .process
            .run(["rev-parse", "--abbrev-ref", "origin/HEAD"], true)
            .await
        {
            if let Some(branch) = extract_branch(result) {
                return Ok(branch);
            }
        };

        if let Ok(result) = self
            .process
            .run(
                ["symbolic-ref", "refs/remotes/origin/HEAD", "--short"],
                true,
            )
            .await
        {
            if let Some(branch) = extract_branch(result) {
                return Ok(branch);
            }
        };

        Ok(self.default_branch.clone())
    }

    #[instrument(skip(self))]
    async fn exec_diff(
        &self,
        module: &GitModule,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let base = self.get_merge_base(base_revision, revision).await?;

        let output = self
            .process
            .run_command(
                self.process.create_command_in_dir(
                    [
                        "--no-pager",
                        "diff",
                        "--name-status",
                        "--no-color",
                        "--relative",
                        "--ignore-submodules",
                        // We use this option so that file names with special characters
                        // are displayed as-is and are not quoted/escaped
                        "-z",
                        base.as_ref().map(|b| b.as_str()).unwrap_or(base_revision),
                    ],
                    module.path.as_str(),
                ),
                false,
            )
            .await?;

        if output.is_empty() {
            return Ok(TouchedFiles::default());
        }

        let mut added = FxHashSet::default();
        let mut deleted = FxHashSet::default();
        let mut modified = FxHashSet::default();
        let mut staged = FxHashSet::default();
        let mut unstaged = FxHashSet::default();
        let mut last_status = "A";

        // Lines AND statuses are terminated by a NUL byte
        //  X\0file\0
        //  X000\0file\0
        //  X000\0file\0renamed_file\0
        for line in output.split('\0') {
            if line.is_empty() {
                continue;
            }

            // X\0
            // X000\0
            if DIFF_SCORE_PATTERN.is_match(line) || DIFF_PATTERN.is_match(line) {
                last_status = &line[0..1];
                continue;
            }

            let x = last_status.chars().next().unwrap_or_default();
            let file = module.path.join(self.to_workspace_relative_path(line));

            match x {
                'A' | 'C' => {
                    added.insert(file.clone());
                    staged.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                    staged.insert(file.clone());
                }
                'M' | 'R' | 'T' => {
                    modified.insert(file.clone());
                    staged.insert(file.clone());
                }
                'U' => {
                    unstaged.insert(file.clone());
                }
                _ => {}
            }
        }

        Ok(TouchedFiles {
            added,
            deleted,
            modified,
            staged,
            unstaged,
            untracked: FxHashSet::default(),
        })
    }

    #[instrument(skip(self))]
    async fn exec_ls_files(
        &self,
        module: &GitModule,
        dir: &str,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut args = vec![
            "ls-files",
            "--full-name",
            "--cached",
            "--modified",
            "--others", // Includes untracked
            "--exclude-standard",
            // This doesn't work with the `--modified` and `--others`
            // flags, so we need to drill into each submodule manually
            // "--recurse-submodules",
        ];

        if self.is_version_supported(">=2.31.0").await? {
            args.push("--deduplicate");
        }

        if !dir.is_empty() {
            args.push(dir);
        }

        let output = self
            .process
            .run_command(
                self.process
                    .create_command_in_dir(args, module.path.as_str()),
                false,
            )
            .await?;

        let paths = output
            .split('\n')
            .filter_map(|file| {
                let path = module.path.join(self.to_workspace_relative_path(file));

                // Do not include directories
                if self.process.workspace_root.join(path.as_str()).is_file() {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(paths)
    }

    // https://git-scm.com/docs/git-status#_short_format
    // Requirements:
    //  Root:
    //    - Run at the root. Does not include submodule files.
    //  Submodule:
    //    - Run in the module root.
    #[instrument(skip(self))]
    async fn exec_status(&self, module: &GitModule) -> miette::Result<TouchedFiles> {
        let output = self
            .process
            .run_command(
                self.process.create_command_in_dir(
                    [
                        "status",
                        "--porcelain",
                        "--untracked-files",
                        // Status does not show files within a submodule, and instead
                        // shows something like `modified: submodules/name (untracked content)`,
                        // so we need to ignore it, and run a status in the submodule directly
                        "--ignore-submodules",
                        // We use this option so that file names with special characters
                        // are displayed as-is and are not quoted/escaped
                        "-z",
                    ],
                    module.path.as_str(),
                ),
                false,
            )
            .await?;

        if output.is_empty() {
            return Ok(TouchedFiles::default());
        }

        let mut added = FxHashSet::default();
        let mut deleted = FxHashSet::default();
        let mut modified = FxHashSet::default();
        let mut untracked = FxHashSet::default();
        let mut staged = FxHashSet::default();
        let mut unstaged = FxHashSet::default();

        // Lines are terminated by a NUL byte:
        //  XY file\0
        //  XY file\0orig_file\0
        for line in output.split('\0') {
            if line.is_empty() {
                continue;
            }

            // orig_file\0
            if !STATUS_PATTERN.is_match(line) {
                continue;
            }

            // XY file\0
            let mut chars = line.chars();
            let x = chars.next().unwrap_or_default();
            let y = chars.next().unwrap_or_default();
            let file = module
                .path
                .join(self.to_workspace_relative_path(&line[3..]));

            match x {
                'A' | 'C' => {
                    added.insert(file.clone());
                    staged.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                    staged.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    staged.insert(file.clone());
                }
                _ => {}
            }

            match y {
                'A' | 'C' => {
                    added.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    unstaged.insert(file.clone());
                }
                '?' => {
                    untracked.insert(file.clone());
                }
                _ => {}
            }
        }

        Ok(TouchedFiles {
            added,
            deleted,
            modified,
            staged,
            unstaged,
            untracked,
        })
    }

    fn to_workspace_relative_path(&self, value: &str) -> WorkspaceRelativePathBuf {
        let file = WorkspaceRelativePathBuf::from(value);

        // Convert the prefixed path back to a workspace relative one...
        if let Some(prefix) = &self.root_prefix {
            if let Ok(rel_file) = file.strip_prefix(prefix) {
                return rel_file.to_owned();
            }
        }

        file
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> miette::Result<Arc<String>> {
        if self.is_version_supported(">=2.22.0").await? {
            return self.process.run(["branch", "--show-current"], true).await;
        }

        self.process
            .run(["rev-parse", "--abbrev-ref", "HEAD"], true)
            .await
    }

    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.process.run(["rev-parse", "HEAD"], true).await
    }

    async fn get_default_branch(&self) -> miette::Result<Arc<String>> {
        Ok(self.default_branch.clone())
    }

    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.process
            .run(["rev-parse", &self.default_branch], true)
            .await
    }

    #[instrument(skip_all)]
    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf], // Workspace relative
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut objects = vec![];
        let mut map = BTreeMap::new();

        for file in files {
            let abs_file = self.process.workspace_root.join(file.as_str());

            // File must exist or git fails
            if abs_file.exists()
                && abs_file.is_file()
                && (allow_ignored || !self.is_ignored(&abs_file))
            {
                // When moon is setup in a sub-folder and not the git root,
                // we need to prefix the paths because `--stdin-paths` assumes
                // the paths are from the git root and don't work correctly...
                if let Some(prefix) = &self.root_prefix {
                    objects.push(prefix.join(file).as_str().to_owned());
                } else {
                    objects.push(file.to_string());
                }
            }
        }

        if objects.is_empty() {
            return Ok(map);
        }

        // Sort for deterministic caching within the vcs layer
        objects.sort();

        let mut command = self
            .process
            .create_command(["hash-object", "--stdin-paths"]);

        command.set_continuous_pipe(true);

        // hash-object requires new lines
        command.input(objects.iter().map(|obj| format!("{obj}\n")));

        let output = self.process.run_command(command, true).await?;

        for (i, hash) in output.split('\n').enumerate() {
            if !hash.is_empty() {
                map.insert(
                    self.to_workspace_relative_path(&objects[i]),
                    hash.to_owned(),
                );
            }
        }

        Ok(map)
    }

    #[instrument(skip(self))]
    async fn get_file_tree(
        &self,
        dir: &RelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        // Check to see if the requested dir is within a submodule
        if let Some(module) = self
            .modules
            .values()
            .find(|module| !module.is_root() && dir.starts_with(&module.path))
        {
            return self
                .exec_ls_files(module, dir.strip_prefix(&module.path).unwrap().as_str())
                .await;
        }

        // If not, then check against the root
        if let Some(module) = self.modules.values().find(|module| module.is_root()) {
            return self.exec_ls_files(module, dir.as_str()).await;
        }

        Ok(vec![])
    }

    async fn get_hooks_dir(&self) -> miette::Result<PathBuf> {
        // Only use the hooks path if it's within the current repository
        let is_in_repo =
            |dir: &Path| dir.is_absolute() && dir.starts_with(self.git_root.parent().unwrap());

        if let Some(tree) = &self.worktree {
            return Ok(tree.git_dir.join("hooks"));
        }

        if let Ok(output) = self
            .process
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

        Ok(self.git_root.join("hooks"))
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        Ok(self
            .worktree
            .as_ref()
            .map(|tree| tree.checkout_dir.as_ref())
            .unwrap_or_else(|| self.git_root.parent().unwrap())
            .to_path_buf())
    }

    async fn get_repository_slug(&self) -> miette::Result<Arc<String>> {
        use git_url_parse::GitUrl;

        for candidate in &self.remote_candidates {
            if let Ok(output) = self
                .process
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

        for result in futures::future::try_join_all(
            self.modules.values().map(|module| self.exec_status(module)),
        )
        .await?
        {
            touched_files.merge(result);
        }

        Ok(touched_files)
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
            .process
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
        for result in futures::future::try_join_all(self.modules.values().filter_map(|module| {
            if module.is_root() {
                Some(self.exec_diff(module, base_revision, revision))
            } else {
                None
            }
        }))
        .await?
        {
            touched_files.merge(result);
        }

        Ok(touched_files)
    }

    async fn get_version(&self) -> miette::Result<Version> {
        let version = self
            .process
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
        self.git_root.exists()
    }

    fn is_ignored(&self, file: &Path) -> bool {
        if let Some(ignore) = &self.ignore {
            ignore.matched(file, false).is_ignore()
        } else {
            false
        }
    }

    async fn is_shallow_checkout(&self) -> miette::Result<bool> {
        let result = if self.is_version_supported(">=2.15.0").await? {
            let result = self
                .process
                .run(["rev-parse", "--is-shallow-repository"], true)
                .await?;

            result.as_str() == "true"
        } else {
            let result = self.process.run(["rev-parse", "--git-dir"], true).await?;

            result.contains("shallow")
        };

        Ok(result)
    }
}
