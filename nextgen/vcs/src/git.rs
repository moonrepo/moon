use crate::process_cache::ProcessCache;
use crate::touched_files::TouchedFiles;
use crate::vcs::Vcs;
use async_trait::async_trait;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::{Diagnostic, IntoDiagnostic};
use moon_common::path::{RelativePathBuf, WorkspaceRelativePathBuf};
use moon_common::{Style, Stylize};
use once_cell::sync::Lazy;
use regex::Regex;
use rustc_hash::FxHashSet;
use semver::Version;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{cmp, env};
use thiserror::Error;
use tracing::debug;

pub static STATUS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(M|T|A|D|R|C|U|\?|!| )(M|T|A|D|R|C|U|\?|!| ) ").unwrap());

pub static DIFF_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(A|D|M|T|U|X)$").unwrap());

pub static DIFF_SCORE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(C|M|R)(\d{3})$").unwrap());

pub static VERSION_CLEAN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\.(windows|win|msysgit|msys)\.\d+").unwrap());

pub fn clean_git_version(version: String) -> String {
    let version = if let Some(index) = version.find('(') {
        &version[0..index]
    } else {
        &version
    };

    VERSION_CLEAN
        .replace(
            version
                .to_lowercase()
                .replace("git", "")
                .replace("version", "")
                .replace("for windows", "")
                .replace("(32-bit)", "")
                .replace("(64-bit)", "")
                .replace("(32bit)", "")
                .replace("(64bit)", "")
                .as_str(),
            "",
        )
        .trim()
        .to_string()
}

#[derive(Error, Debug, Diagnostic)]
pub enum GitError {
    #[diagnostic(code(git::ignore::load_invalid))]
    #[error("Failed to load and parse {}.", ".gitignore".style(Style::File))]
    GitignoreLoadFailed {
        #[source]
        error: ignore::Error,
    },

    #[diagnostic(code(git::repository::extract_slug))]
    #[error("Failed to extract a repository slug from git remote candidates.")]
    ExtractRepoSlugFailed,
}

#[derive(Debug)]
pub struct Git {
    /// Default git branch name.
    pub default_branch: String,

    /// Ignore rules derived from a root `.gitignore` file.
    ignore: Option<Gitignore>,

    /// Run and cache `git` commands.
    pub process: ProcessCache,

    /// List of remotes to use as merge candidates.
    pub remote_candidates: Vec<String>,

    /// Root of the git repository (where `.git` directory is located).
    pub repository_root: PathBuf,

    /// Path between the git and workspace root.
    pub root_prefix: RelativePathBuf,

    /// If in a git worktree, the root of the worktree (the `.git` file).
    pub worktree_root: Option<PathBuf>,
}

impl Git {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
        remote_candidates: &[String],
    ) -> miette::Result<Git> {
        debug!("Using git as a version control system");

        let workspace_root = workspace_root.as_ref();

        debug!(
            starting_dir = ?workspace_root,
            "Attempting to find a .git directory or file"
        );

        // Find the .git dir
        let mut current_dir = workspace_root;
        let mut repository_root = workspace_root.to_path_buf();
        let mut worktree_root = None;

        loop {
            let git_dir = current_dir.join(".git");

            if git_dir.exists() {
                if git_dir.is_file() {
                    debug!(
                        git_dir = ?git_dir,
                        "Found a .git file (worktree root), continuing search"
                    );

                    worktree_root = Some(current_dir.to_path_buf());
                } else {
                    debug!(
                        git_dir = ?git_dir,
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

                    current_dir = workspace_root;
                    break;
                }
            };
        }

        // Load .gitignore
        let mut ignore: Option<Gitignore> = None;
        let ignore_path = current_dir.join(".gitignore");

        if ignore_path.exists() {
            debug!(
                ignore_file = ?ignore_path,
                "Loading ignore rules from .gitignore",
            );

            let mut builder = GitignoreBuilder::new(current_dir);

            if let Some(error) = builder.add(ignore_path) {
                return Err(GitError::GitignoreLoadFailed { error }.into());
            }

            ignore = Some(
                builder
                    .build()
                    .map_err(|error| GitError::GitignoreLoadFailed { error })?,
            );
        }

        let active_dir = worktree_root.as_ref().unwrap_or(&repository_root);

        Ok(Git {
            default_branch: default_branch.as_ref().to_owned(),
            ignore,
            remote_candidates: remote_candidates.to_owned(),
            root_prefix: if active_dir == workspace_root {
                RelativePathBuf::default()
            } else {
                RelativePathBuf::from_path(workspace_root.strip_prefix(active_dir).unwrap())
                    .into_diagnostic()?
            },
            process: ProcessCache::new("git", workspace_root),
            repository_root,
            worktree_root,
        })
    }

    async fn get_merge_base(&self, base: &str, head: &str) -> miette::Result<Option<&str>> {
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

    fn get_working_root(&self) -> &Path {
        self.worktree_root.as_ref().unwrap_or(&self.repository_root)
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> miette::Result<&str> {
        if self.is_version_supported(">=2.22.0").await? {
            return self.process.run(["branch", "--show-current"], true).await;
        }

        self.process
            .run(["rev-parse", "--abbrev-ref", "HEAD"], true)
            .await
    }

    async fn get_local_branch_revision(&self) -> miette::Result<&str> {
        self.process.run(["rev-parse", "HEAD"], true).await
    }

    async fn get_default_branch(&self) -> miette::Result<&str> {
        Ok(&self.default_branch)
    }

    async fn get_default_branch_revision(&self) -> miette::Result<&str> {
        self.process
            .run(["rev-parse", &self.default_branch], true)
            .await
    }

    async fn get_file_hashes(
        &self,
        files: &[String], // Workspace relative
        allow_ignored: bool,
        batch_size: u16,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut objects = vec![];
        let mut map = BTreeMap::new();
        let is_not_root = self.process.root != self.get_working_root();

        for file in files {
            let abs_file = self.process.root.join(file);

            // File must exist or git fails
            if abs_file.exists()
                && abs_file.is_file()
                && (allow_ignored || !self.is_ignored(&abs_file))
            {
                // When moon is setup in a sub-folder and not the git root,
                // we need to prefix the paths because `--stdin-paths` assumes
                // the paths are from the git root and don't work correctly...
                if is_not_root {
                    objects.push(self.root_prefix.join(file).as_str().to_owned());
                } else {
                    objects.push(file.to_owned());
                }
            }
        }

        if objects.is_empty() {
            return Ok(map);
        }

        // Sort for deterministic caching within the vcs layer
        objects.sort();

        // Chunk into slices to avoid passing too many files
        let mut index = 0;
        let end_index = objects.len();

        while index < end_index {
            let next_index = cmp::min(index + (batch_size as usize), end_index);
            let slice = objects[index..next_index].to_vec();

            let mut command = self
                .process
                .create_command(["hash-object", "--stdin-paths"]);
            command.input([slice.join("\n")]);

            let output = self.process.run_command(command, true).await?;

            for (i, hash) in output.split('\n').enumerate() {
                if !hash.is_empty() {
                    let mut file = WorkspaceRelativePathBuf::from(&slice[i]);

                    // Convert the prefixed path back to a workspace relative one...
                    if is_not_root {
                        file = file.strip_prefix(&self.root_prefix).unwrap().to_owned();
                    }

                    map.insert(file, hash.to_owned());
                }
            }

            index = next_index;
        }

        Ok(map)
    }

    async fn get_file_tree(&self, dir: &str) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let mut args = vec![
            "ls-files",
            "--full-name",
            "--cached",
            "--modified",
            "--others", // Includes untracked
            "--exclude-standard",
            dir,
        ];

        if self.is_version_supported(">=2.31.0").await? {
            args.push("--deduplicate");
        }

        let output = self.process.run(args, true).await?;

        Ok(output
            .split('\n')
            .map(WorkspaceRelativePathBuf::from)
            .collect::<Vec<_>>())
    }

    async fn get_hooks_dir(&self) -> miette::Result<PathBuf> {
        if let Ok(output) = self
            .process
            .run(["config", "--get", "core.hooksPath"], true)
            .await
        {
            return Ok(PathBuf::from(output));
        }

        if let Ok(dir) = env::var("GIT_DIR") {
            return Ok(PathBuf::from(dir).join("hooks"));
        }

        Ok(self.repository_root.join(".git").join("hooks"))
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        Ok(self.repository_root.to_owned())
    }

    async fn get_repository_slug(&self) -> miette::Result<&str> {
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

    // https://git-scm.com/docs/git-status#_short_format
    async fn get_touched_files(&self) -> miette::Result<TouchedFiles> {
        let output = self
            .process
            .run(
                [
                    "status",
                    "--porcelain",
                    "--untracked-files",
                    // We use this option so that file names with special characters
                    // are displayed as-is and are not quoted/escaped
                    "-z",
                ],
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
            let file = WorkspaceRelativePathBuf::from(&line[3..]);

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

    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let revision = if self.is_default_branch(revision) {
            "HEAD"
        } else {
            revision
        };

        self.get_touched_files_between_revisions(format!("{revision}~1").as_str(), revision)
            .await
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let base = self
            .get_merge_base(base_revision, revision)
            .await?
            .unwrap_or(base_revision);

        let output = self
            .process
            .run(
                [
                    "--no-pager",
                    "diff",
                    "--name-status",
                    "--no-color",
                    "--relative",
                    // We use this option so that file names with special characters
                    // are displayed as-is and are not quoted/escaped
                    "-z",
                    base,
                ],
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
            let file = WorkspaceRelativePathBuf::from(line);

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

    async fn get_version(&self) -> miette::Result<Version> {
        let version = self
            .process
            .run_with_formatter(["--version"], true, clean_git_version)
            .await?;

        Ok(Version::parse(version).unwrap())
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        let default_branch = &self.default_branch;

        if default_branch == branch {
            return true;
        }

        if default_branch.contains('/') {
            return default_branch.ends_with(&format!("/{branch}"));
        }

        false
    }

    fn is_enabled(&self) -> bool {
        self.get_working_root().join(".git").exists()
    }

    fn is_ignored(&self, file: &Path) -> bool {
        if let Some(ignore) = &self.ignore {
            ignore.matched(file, false).is_ignore()
        } else {
            false
        }
    }
}
