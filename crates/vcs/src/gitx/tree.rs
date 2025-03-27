use super::common::*;
use crate::process_cache::ProcessCache;
use crate::touched_files::TouchedFiles;
use moon_common::path::RelativePathBuf;
use rustc_hash::FxHashSet;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::instrument;

#[derive(PartialEq)]
pub enum GitTreeType {
    Root,
    Submodule,
    Worktree,

    // There's no markers in the repository that denotes a folder as a subtree,
    // as the subtree commit history is squashed/committed into the main repository.
    // At this point, it just looks like a normal folder. Nothing to do here?
    Subtree,
}

pub struct GitTree {
    /// Absolute path to the tree's `.git` directory.
    ///   Root -> /.git
    ///   Submodule -> /.git/modules/submodules/<name>
    ///   Worktree -> /.git/worktrees/<name>
    pub git_dir: PathBuf,

    /// Absolute path to the tree root. The working directory for this tree.
    pub work_dir: PathBuf,

    /// Relative path from the repository root to the tree root.
    ///   Submodule -> .gitmodules
    pub path: RelativePathBuf,

    /// Process runner and caching.
    pub process: Arc<ProcessCache>,

    /// The type of tree.
    pub ty: GitTreeType,
}

impl GitTree {
    pub fn is_root(&self) -> bool {
        self.ty == GitTreeType::Root
    }

    pub fn is_submodule(&self) -> bool {
        self.ty == GitTreeType::Submodule
    }

    pub fn is_subtree(&self) -> bool {
        self.ty == GitTreeType::Subtree
    }

    pub fn is_worktree(&self) -> bool {
        self.ty == GitTreeType::Worktree
    }

    // https://git-scm.com/docs/git-diff
    //
    // Requirements:
    //  Root:
    //    - Run at the root.
    //    - Does not include submodule files.
    //  Submodule:
    //    - Run in the module root.
    #[instrument(skip(self))]
    async fn exec_diff(
        &self,
        base_revision: &str,
        merge_revision: Option<&str>,
    ) -> miette::Result<TouchedFiles> {
        let output = self
            .process
            .run_command(
                self.process.create_command_in_cwd(
                    [
                        "--no-pager",
                        "diff",
                        "--name-status",
                        "--no-color",
                        "--relative",
                        // Ignore submodules since they would have different revisions
                        "--ignore-submodules",
                        // We use this option so that file names with special characters
                        // are displayed as-is and are not quoted/escaped
                        "-z",
                        merge_revision.as_deref().unwrap_or(base_revision),
                    ],
                    &self.work_dir,
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

            // Paths are relative from the cwd
            let file = self.path.join(line);

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

    // https://git-scm.com/docs/git-ls-files
    //
    // Requirements:
    //  Root:
    //    - Run at the root.
    //    - Includes submodule dir in this list, which causes problems.
    //  Submodule:
    //    - Run in the module root.
    #[instrument(skip(self))]
    async fn exec_ls_files(&self) -> miette::Result<Vec<RelativePathBuf>> {
        let output = self
            .process
            .run_command(
                self.process.create_command_in_cwd(
                    [
                        "ls-files",
                        "--full-name",
                        "--cached",
                        "--modified",
                        // Includes untracked files
                        "--others",
                        "--exclude-standard",
                        // This doesn't work with the `--modified` and `--others`
                        // flags, so we need to drill into each submodule manually
                        // "--recurse-submodules",
                        ".",
                    ],
                    &self.work_dir,
                ),
                false,
            )
            .await?;

        let paths = output
            .split('\n')
            .filter_map(|file| {
                // Paths are relative from the cwd
                let path = self.path.join(file);

                // Do not include directories, which will be included in this list
                // when git encounters a submodule
                if self.process.root.join(path.as_str()).is_file() {
                    Some(path)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(paths)
    }

    // https://git-scm.com/docs/git-status#_short_format
    //
    // Requirements:
    //  Root:
    //    - Run at the root.
    //    - Does not include submodule files.
    //  Submodule:
    //    - Run in the module root.
    #[instrument(skip(self))]
    async fn exec_status(&self) -> miette::Result<TouchedFiles> {
        let output = self
            .process
            .run_command(
                self.process.create_command_in_cwd(
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
            let x = chars.next().unwrap_or_default(); // 0
            let y = chars.next().unwrap_or_default(); // 1

            // Paths are relative from the cwd
            let file = self.path.join(&line[3..]);

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
}
