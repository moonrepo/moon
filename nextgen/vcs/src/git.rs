use crate::touched_files::TouchedFiles;
use crate::vcs::{Vcs, VcsResult};
use crate::vcs_error::VcsError;
use async_trait::async_trait;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use moon_process::{output_to_string, Command};
use once_cell::sync::Lazy;
use once_map::OnceMap;
use regex::Regex;
use relative_path::RelativePathBuf;
use rustc_hash::FxHashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use tracing::debug;

pub static STATUS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(M|T|A|D|R|C|U|\?|!| )(M|T|A|D|R|C|U|\?|!| ) ").unwrap());

pub static DIFF_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(A|D|M|T|U|X)$").unwrap());

pub static DIFF_SCORE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(C|M|R)(\d{3})$").unwrap());

pub struct Git {
    /// Output cache of all executed git commands.
    cache: OnceMap<String, String>,

    /// Default git branch name.
    default_branch: String,

    /// Path between the git and workspace root.
    file_prefix: RelativePathBuf,

    /// Ignore rules derived from a root `.gitignore` file.
    ignore: Option<Gitignore>,

    /// Root of the git repository (where `.git` is located).
    git_root: PathBuf,

    /// List of remotes to use as merge candidates.
    remote_candidates: Vec<String>,

    /// Root of the moon workspace.
    workspace_root: PathBuf,
}

impl Git {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
    ) -> VcsResult<Git> {
        debug!("Using git as a VCS");

        let workspace_root = workspace_root.as_ref();

        debug!(
            starting_dir = %workspace_root.display(),
            "Attempting to find a .git directory"
        );

        // Find the .git dir
        let mut git_root = workspace_root;

        loop {
            if git_root.join(".git").exists() {
                debug!(
                    git_root = %git_root.display(),
                    "Found a .git directory"
                );

                break;
            }

            match git_root.parent() {
                Some(parent) => git_root = parent,
                None => {
                    debug!("Unable to find .git, falling back to workspace root");

                    git_root = workspace_root;
                    break;
                }
            };
        }

        // Load .gitignore
        let mut ignore: Option<Gitignore> = None;
        let ignore_path = git_root.join(".gitignore");

        if ignore_path.exists() {
            debug!(
                ignore_path = %ignore_path.display(),
                "Loading ignore rules from .gitignore",
            );

            let mut builder = GitignoreBuilder::new(&git_root);

            if let Some(error) = builder.add(ignore_path) {
                return Err(VcsError::LoadGitignoreFailed { error });
            }

            ignore = Some(
                builder
                    .build()
                    .map_err(|error| VcsError::LoadGitignoreFailed { error })?,
            );
        }

        Ok(Git {
            cache: OnceMap::new(),
            default_branch: default_branch.as_ref().to_owned(),
            ignore,
            file_prefix: RelativePathBuf::from_path(workspace_root.strip_prefix(git_root).unwrap())
                .unwrap(),
            remote_candidates: Vec::new(),
            git_root: git_root.to_owned(),
            workspace_root: workspace_root.to_owned(),
        })
    }

    fn create_command<I, A>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        let mut command = Command::new("git");
        command.args(args);
        // Run from workspace root instead of git root so that we can avoid
        // prefixing all file paths to ensure everything is relative and accurate.
        command.cwd(&self.workspace_root);
        command
    }

    async fn create_and_run_command<I, A>(&self, args: I, trim: bool) -> VcsResult<String>
    where
        I: IntoIterator<Item = A>,
        A: AsRef<OsStr>,
    {
        self.run_command(self.create_command(args), trim).await
    }

    async fn run_command(&self, command: Command, trim: bool) -> VcsResult<String> {
        let mut executor = command.create_async();
        let cache_key = executor.inspector.get_cache_key();

        // Execute and insert output into the cache if not already present
        if !self.cache.contains_key(&cache_key) {
            let output = executor.exec_capture_output().await?;

            self.cache
                .insert(cache_key.clone(), |_| output_to_string(&output.stdout));
        }

        let output = self.cache.get(&cache_key).unwrap();

        Ok(if trim {
            output.trim().to_owned()
        } else {
            output.to_owned()
        })
    }

    async fn get_merge_base(&self, base: &str, head: &str) -> VcsResult<String> {
        let mut args = vec!["merge-base", head];
        let mut candidates = vec![base.to_owned()];

        for remote in &self.remote_candidates {
            candidates.push(format!("{remote}/{base}"));
        }

        // To start, we need to find a working base
        for candidate in &candidates {
            if self
                .create_and_run_command(["merge-base", &candidate, head], true)
                .await
                .is_ok()
            {
                args.push(candidate);
            }
        }

        // Then we need to run it again and extract the base hash.
        // This is necessary to support comparisons between forks!
        if let Ok(hash) = self.create_and_run_command(args, true).await {
            return Ok(hash);
        }

        Ok(base.to_owned())
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> VcsResult<String> {
        // --show-current was added in 2.22.0
        if let Ok(branch) = self
            .create_and_run_command(["branch", "--show-current"], true)
            .await
        {
            return Ok(branch);
        }

        self.create_and_run_command(["rev-parse --abbrev-ref HEAD"], true)
            .await
    }

    async fn get_local_branch_revision(&self) -> VcsResult<String> {
        self.create_and_run_command(["rev-parse", "HEAD"], true)
            .await
    }

    fn get_default_branch(&self) -> &str {
        &self.default_branch
    }

    async fn get_default_branch_revision(&self) -> VcsResult<String> {
        self.create_and_run_command(["rev-parse", &self.default_branch], true)
            .await
    }

    async fn get_repository_slug(&self) -> VcsResult<String> {
        let output = self
            .create_and_run_command(["remote", "get-url", "origin"], true)
            .await?;

        // TODO
        // Self::extract_slug_from_remote(output)
        Ok(output)
    }

    // https://git-scm.com/docs/git-status#_short_format
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self
            .create_and_run_command(
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
            let file = RelativePathBuf::from(&line[3..]);

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
    ) -> VcsResult<TouchedFiles> {
        let revision = if self.is_default_branch(revision) {
            "HEAD"
        } else {
            revision
        };

        Ok(self
            .get_touched_files_between_revisions(format!("{revision}~1").as_str(), revision)
            .await?)
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> VcsResult<TouchedFiles> {
        let base = self.get_merge_base(base_revision, revision).await?;

        let output = self
            .create_and_run_command(
                [
                    "--no-pager",
                    "diff",
                    "--name-status",
                    "--no-color",
                    "--relative",
                    // We use this option so that file names with special characters
                    // are displayed as-is and are not quoted/escaped
                    "-z",
                    &base,
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
            let file = RelativePathBuf::from(line);

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
        self.git_root.join(".git").exists()
    }

    fn is_ignored(&self, file: &str) -> bool {
        if let Some(ignore) = &self.ignore {
            ignore.matched(file, false).is_ignore()
        } else {
            false
        }
    }
}
