use super::common::*;
use super::git_error::GitError;
use crate::changed_files::*;
use crate::process_cache::ProcessCache;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::IntoDiagnostic;
use moon_common::path::{RelativePath, RelativePathBuf};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::{debug, instrument};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum GitTreeType {
    #[default]
    Root,
    Submodule,
    Worktree,

    // There's no markers in the repository that denotes a folder as a subtree,
    // as the subtree commit history is squashed/committed into the main repository.
    // At this point, it just looks like a normal folder. Nothing to do here?
    #[allow(dead_code)]
    Subtree,
}

#[derive(Clone, Default)]
pub struct GitTree {
    /// Absolute path to the tree's `.git` directory.
    ///   Root -> /.git
    ///   Submodule -> /.git/modules/submodules/<name>
    ///   Worktree -> /.git/worktrees/<name>
    pub git_dir: PathBuf,

    /// Ignore rules derived from a `.gitignore` file.
    pub ignore: Option<Arc<Gitignore>>,

    /// Relative path from the worktree root to the this tree root.
    pub path: RelativePathBuf,

    /// Process runner and caching.
    pub process: Option<Arc<ProcessCache>>,

    /// The type of tree.
    pub type_of: GitTreeType,

    /// Absolute path to the tree root. The working directory for this tree.
    pub work_dir: PathBuf,
}

impl GitTree {
    pub fn load_ignore(&mut self) -> miette::Result<()> {
        let ignore_path = self.work_dir.join(".gitignore");

        if ignore_path.exists() {
            debug!(
                ignore_file = ?ignore_path,
                "Loading ignore rules from .gitignore",
            );

            let mut builder = GitignoreBuilder::new(&self.work_dir);

            if let Some(error) = builder.add(&ignore_path) {
                return Err(GitError::IgnoreLoadFailed {
                    path: ignore_path,
                    error: Box::new(error),
                }
                .into());
            }

            self.ignore = Some(Arc::new(builder.build().map_err(|error| {
                GitError::IgnoreLoadFailed {
                    path: ignore_path,
                    error: Box::new(error),
                }
            })?));
        }

        Ok(())
    }

    pub fn is_ignored(&self, file: &Path) -> bool {
        if let Some(ignore) = &self.ignore {
            ignore.matched(file, file.is_dir()).is_ignore()
        } else {
            false
        }
    }

    pub fn is_root(&self) -> bool {
        self.type_of == GitTreeType::Root
    }

    pub fn is_submodule(&self) -> bool {
        self.type_of == GitTreeType::Submodule
    }

    pub fn is_subtree(&self) -> bool {
        self.type_of == GitTreeType::Subtree
    }

    pub fn is_worktree(&self) -> bool {
        self.type_of == GitTreeType::Worktree
    }

    pub fn get_process(&self) -> &ProcessCache {
        self.process.as_deref().unwrap()
    }

    // https://git-scm.com/docs/git-diff
    //
    // Requirements:
    //  Root/Worktree:
    //    - Run at the root.
    //    - Does not include submodule files.
    //  Submodule:
    //    - Run in the module root.
    #[instrument(skip(self))]
    pub async fn exec_diff(
        &self,
        base_revision: &str,
        head_revision: &str,
    ) -> miette::Result<ChangedFiles<PathBuf>> {
        validate_revision(base_revision)?;
        validate_revision(head_revision)?;

        let process = self.get_process();
        let mut args = vec![
            "diff",
            "--name-status",
            "--no-color",
            "--relative",
            // Ignore submodules since they would have different revisions
            "--ignore-submodules",
            // Disable rename detection, as it requires content comparisons
            // (slow, and triggers blob fetches in partial clones). Renames
            // are instead reported as a deletion and an addition, which is
            // more accurate for change detection anyway
            "--no-renames",
            // We use this option so that file names with special characters
            // are displayed as-is and are not quoted/escaped
            "-z",
            base_revision,
        ];

        if !head_revision.is_empty() {
            args.push(head_revision);
        }

        let output = process
            .run_command(process.create_command_in_cwd(args, &self.work_dir), false)
            .await?;

        if output.is_empty() {
            return Ok(ChangedFiles::default());
        }

        let mut files = FxHashMap::default();
        let mut tokens = output.split('\0');

        // Statuses AND paths are terminated by a NUL byte, and strictly
        // alternate, so consume them in pairs (renames and copies are
        // disabled above, so a status is always followed by one path)
        //  X\0file\0
        while let Some(token) = tokens.next() {
            if token.is_empty() {
                continue;
            }

            // X\0
            if !DIFF_PATTERN.is_match(token) {
                continue;
            }

            let x = token.chars().next().unwrap_or_default();
            let mut statuses = vec![];

            match x {
                'A' => {
                    statuses.push(ChangedStatus::Added);
                    statuses.push(ChangedStatus::Staged);
                }
                'D' => {
                    statuses.push(ChangedStatus::Deleted);
                    statuses.push(ChangedStatus::Staged);
                }
                'M' | 'T' => {
                    statuses.push(ChangedStatus::Modified);
                    statuses.push(ChangedStatus::Staged);
                }
                'U' => {
                    statuses.push(ChangedStatus::Unstaged);
                }
                _ => {}
            }

            if let Some(path) = tokens.next()
                && !path.is_empty()
            {
                // Paths are relative from the cwd
                files.insert(self.work_dir.join(path), statuses);
            }
        }

        Ok(ChangedFiles { files })
    }

    // Hash the empty tree, respecting the repository's object format
    // (sha1 or sha256). Used as a diff base when no other revision exists.
    #[instrument(skip(self))]
    pub async fn exec_hash_empty_tree(&self) -> miette::Result<Arc<String>> {
        let process = self.get_process();
        let mut command =
            process.create_command_in_cwd(["hash-object", "-t", "tree", "--stdin"], &self.work_dir);
        command.input([""]);

        process.run_command(command, true).await
    }

    // https://git-scm.com/docs/git-ls-files
    //
    // Requirements:
    //  Root/Worktree:
    //    - Run at the worktree root.
    //    - Includes submodule dir in this list, which causes problems as
    //      we need files, so filter it out.
    //  Submodule:
    //    - Run in the submodule root.
    #[instrument(skip(self))]
    pub async fn exec_ls_files(&self, dir: &RelativePath) -> miette::Result<Vec<PathBuf>> {
        let process = self.get_process();
        let mut args = vec![
            "ls-files",
            "--full-name",
            "--cached",
            "--modified",
            // Includes untracked files
            "--others",
            "--exclude-standard",
            // We use this option so that file names with special characters
            // are displayed as-is and are not quoted/escaped
            "-z",
            // This doesn't work with the `--modified` and `--others`
            // flags, so we need to drill into each submodule manually
            // "--recurse-submodules",
        ];

        if dir.as_str().is_empty() {
            args.push(".");
        } else {
            args.push(dir.as_str());
        }

        let output = process
            .run_command(process.create_command_in_cwd(args, &self.work_dir), false)
            .await?;

        let mut paths = vec![];
        let mut seen = FxHashSet::default();

        for file in output.split('\0') {
            // Files are listed once for each criteria above that they
            // match, so any duplicates must be filtered out
            if file.is_empty() || !seen.insert(file) {
                continue;
            }

            // Paths are relative from the cwd
            let path = self.work_dir.join(file);

            // Do not include directories, which will be included in this list
            // when git encounters a submodule (it doesn't traverse into it)
            if path.is_file() {
                paths.push(path);
            }
        }

        Ok(paths)
    }

    // https://git-scm.com/docs/git-ls-tree
    //
    // Extracts submodule (gitlink) entries at the provided paths, mapping
    // the absolute path of each submodule to its commit hash. We pass the
    // paths as pathspecs instead of recursing with `-r`, as the latter
    // would list every file in the repository.
    //
    // Requirements:
    //  Root/Worktree:
    //    - Run at the worktree root, with paths relative from it.
    #[instrument(skip(self))]
    pub async fn exec_ls_tree(
        &self,
        revision: &str,
        paths: &[&str],
    ) -> miette::Result<BTreeMap<PathBuf, String>> {
        validate_revision(revision)?;

        let process = self.get_process();
        let mut args = vec!["ls-tree", "-z", revision, "--"];
        args.extend(paths.iter().copied());

        let output = process
            .run_command(process.create_command_in_cwd(args, &self.work_dir), false)
            .await?;

        let mut tree = BTreeMap::default();

        // Lines are formatted as:
        //  <mode> <type> <hash>\t<path>\0
        for line in output.split('\0') {
            if line.is_empty() {
                continue;
            }

            // Split on the tab first, as the path may contain spaces
            if let Some((meta, path)) = line.split_once('\t')
                && let Some((mode_type, hash)) = meta.rsplit_once(' ')
                // Only include submodule entries, as a path may point to
                // a regular directory in revisions before it was added
                && mode_type.ends_with(" commit")
            {
                tree.insert(self.work_dir.join(path), hash.to_owned());
            }
        }

        Ok(tree)
    }

    #[instrument(skip(self))]
    pub async fn exec_merge_base(
        &self,
        base_revision: &str,
        head_revision: &str,
        remote_candidates: &[String],
    ) -> miette::Result<Option<Arc<String>>> {
        validate_revision(base_revision)?;
        validate_revision(head_revision)?;

        let mut candidates = vec![base_revision.to_owned()];

        for remote in remote_candidates {
            candidates.push(format!("{remote}/{base_revision}"));
        }

        // To start, find all the candidates that share history
        // with the head, and extract their merge base
        let mut set = JoinSet::new();
        let mut resolved = vec![None; candidates.len()];

        for (index, candidate) in candidates.into_iter().enumerate() {
            let process = Arc::clone(self.process.as_ref().unwrap());
            let command = process
                .create_command_in_cwd(["merge-base", &candidate, head_revision], &self.work_dir);

            set.spawn(async move {
                (
                    index,
                    process
                        .run_command(command, true)
                        .await
                        .map(|hash| (candidate, hash)),
                )
            });
        }

        // Tasks complete in any order, so preserve the candidate
        // order to keep the final command deterministic
        while let Some(result) = set.join_next().await {
            let (index, result) = result.into_diagnostic()?;

            if let Ok(pair) = result {
                resolved[index] = Some(pair);
            }
        }

        let mut resolved = resolved.into_iter().flatten();

        // No candidates resolved, so a merge base can't be found
        let Some((first_candidate, first_hash)) = resolved.next() else {
            return Ok(None);
        };

        let other_candidates = resolved.map(|(candidate, _)| candidate).collect::<Vec<_>>();

        // Only 1 candidate resolved, so reuse the merge base from its probe
        if other_candidates.is_empty() {
            return Ok(Some(first_hash));
        }

        // Otherwise run it again with all the viable candidates. When given
        // multiple commits, Git computes the merge base between the head and
        // a hypothetical merge of all the others, which surfaces the most
        // recent divergence point when the candidates are out of sync. This
        // is necessary to support stale local branches and forks!
        let mut args = vec![
            "merge-base".to_owned(),
            head_revision.to_owned(),
            first_candidate,
        ];
        args.extend(other_candidates);

        let process = self.get_process();

        if let Ok(hash) = process
            .run_command(process.create_command_in_cwd(args, &self.work_dir), true)
            .await
        {
            return Ok(Some(hash));
        }

        // The combined command failed unexpectedly, so fall
        // back to the first candidate's merge base
        Ok(Some(first_hash))
    }

    // https://git-scm.com/docs/git-status#_short_format
    //
    // Requirements:
    //  Root:
    //    - Run at the worktree root.
    //    - Does not include submodule files.
    //  Submodule:
    //    - Run in the submodule root.
    #[instrument(skip(self))]
    pub async fn exec_status(&self) -> miette::Result<ChangedFiles<PathBuf>> {
        let process = self.get_process();
        let output = process
            .run_command(
                process.create_command_in_cwd(
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
                    &self.work_dir,
                ),
                false,
            )
            .await?;

        if output.is_empty() {
            return Ok(ChangedFiles::default());
        }

        let mut files = FxHashMap::default();
        let mut tokens = output.split('\0');

        // Lines are terminated by a NUL byte, and rename/copy entries
        // are followed by the original path as a separate token:
        //  XY file\0
        //  XY file\0orig_file\0
        while let Some(token) = tokens.next() {
            if token.is_empty() {
                continue;
            }

            // Unknown token (the original path should be consumed below)
            if !STATUS_PATTERN.is_match(token) {
                continue;
            }

            // XY file\0
            let mut chars = token.chars();
            let x = chars.next().unwrap_or_default(); // 0
            let y = chars.next().unwrap_or_default(); // 1

            // Paths are relative from the cwd
            let file = self.work_dir.join(&token[3..]);
            let mut statuses = vec![];

            match x {
                'A' | 'C' => {
                    statuses.push(ChangedStatus::Added);
                    statuses.push(ChangedStatus::Staged);
                }
                'D' => {
                    statuses.push(ChangedStatus::Deleted);
                    statuses.push(ChangedStatus::Staged);
                }
                'M' | 'R' | 'T' => {
                    statuses.push(ChangedStatus::Modified);
                    statuses.push(ChangedStatus::Staged);
                }
                // Unmerged (conflicted)
                'U' => {
                    statuses.push(ChangedStatus::Unstaged);
                }
                _ => {}
            }

            match y {
                'A' | 'C' => {
                    statuses.push(ChangedStatus::Added);
                    statuses.push(ChangedStatus::Unstaged);
                }
                'D' => {
                    statuses.push(ChangedStatus::Deleted);
                    statuses.push(ChangedStatus::Unstaged);
                }
                'M' | 'R' | 'T' => {
                    statuses.push(ChangedStatus::Modified);
                    statuses.push(ChangedStatus::Unstaged);
                }
                // Unmerged (conflicted)
                'U' => {
                    statuses.push(ChangedStatus::Unstaged);
                }
                '?' => {
                    statuses.push(ChangedStatus::Untracked);
                }
                _ => {}
            }

            // Renames and copies are followed by the original path, so
            // consume it. For renames the original path no longer exists,
            // so also mark it as deleted.
            if (x == 'R' || x == 'C' || y == 'R' || y == 'C')
                && let Some(orig) = tokens.next()
                && !orig.is_empty()
                && (x == 'R' || y == 'R')
            {
                files.insert(
                    self.work_dir.join(orig),
                    vec![
                        ChangedStatus::Deleted,
                        if x == 'R' {
                            ChangedStatus::Staged
                        } else {
                            ChangedStatus::Unstaged
                        },
                    ],
                );
            }

            files.insert(file, statuses);
        }

        Ok(ChangedFiles { files })
    }
}

impl fmt::Debug for GitTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GitTree")
            .field("git_dir", &self.git_dir)
            .field("path", &self.path)
            .field("type_of", &self.type_of)
            .field("work_dir", &self.work_dir)
            .finish()
    }
}

impl PartialEq for GitTree {
    fn eq(&self, other: &Self) -> bool {
        self.git_dir == other.git_dir
            && self.path == other.path
            && self.type_of == other.type_of
            && self.work_dir == other.work_dir
    }
}
