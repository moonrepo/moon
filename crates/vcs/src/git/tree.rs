use super::common::*;
use super::git_error::GitError;
use crate::changed_files::*;
use crate::process_cache::ProcessCache;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::IntoDiagnostic;
use moon_common::path::{RelativePath, RelativePathBuf};
use rustc_hash::FxHashMap;
use starbase_utils::fs;
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
    pub fn load(repository_root: &Path, bare: bool) -> miette::Result<Self> {
        Ok(Self {
            git_dir: if bare {
                repository_root.to_owned()
            } else {
                repository_root.join(".git")
            },
            type_of: GitTreeType::Root,
            work_dir: repository_root.to_owned(),
            ..Default::default()
        })
    }

    pub fn load_git_file(work_dir: &Path) -> miette::Result<PathBuf> {
        let contents = fs::read_file(work_dir.join(".git"))?;

        for line in contents.lines() {
            if let Some(suffix) = line.strip_prefix("gitdir:") {
                let mut dir = PathBuf::from(suffix.trim());

                if !dir.is_absolute() {
                    dir = work_dir.join(dir);
                }

                return dir.canonicalize().map_err(|error| {
                    GitError::LoadGitDirFailed {
                        path: dir,
                        error: Box::new(error),
                    }
                    .into()
                });
            }
        }

        Err(GitError::ParseGitFileFailed {
            path: work_dir.join(".git"),
        }
        .into())
    }

    pub fn load_submodules(worktree_root: &Path) -> miette::Result<Vec<Self>> {
        let mut modules = vec![];
        let gitmodules_file = worktree_root.join(".gitmodules");

        if !gitmodules_file.exists() {
            return Ok(modules);
        }

        debug!(
            modules_file = ?gitmodules_file,
            "Loading submodules from .gitmodules",
        );

        let mut current_module_name = None;
        let mut current_module = Self::default();
        let contents = fs::read_file(gitmodules_file)?;

        fn clean_line(line: &str) -> String {
            line.replace("=", "").replace("\"", "").trim().to_owned()
        }

        for line in contents.lines() {
            let line = line.trim();

            if line.starts_with("[submodule") {
                if current_module_name.is_some() {
                    modules.push(current_module);
                    current_module = Self::default();
                }

                current_module_name = Some(
                    line.replace("[submodule", "")
                        .replace("\"", "")
                        .replace("]", "")
                        .trim()
                        .to_owned(),
                );
            } else if let Some(value) = line.strip_prefix("path") {
                current_module.path = RelativePathBuf::from(clean_line(value));
            }
        }

        if current_module_name.is_some() {
            modules.push(current_module);
        }

        // Filter out invalid modules
        let mut modules = modules
            .into_iter()
            .filter_map(|mut module| {
                let rel_path = module.path.as_str();

                if rel_path.is_empty() {
                    None
                } else {
                    module.work_dir = worktree_root.join(rel_path);
                    module.type_of = GitTreeType::Submodule;

                    // Ensure the submodule has been checked out
                    if module.work_dir.join(".git").exists() {
                        Some(module)
                    } else {
                        debug!(
                            submodule = ?module.work_dir,
                            "Encountered a submodule that hasn't been checked out, skipping it"
                        );

                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        // Extract the git dirs
        for module in &mut modules {
            module.git_dir = Self::load_git_file(&module.work_dir)?;
        }

        Ok(modules)
    }

    pub fn load_worktree(worktree_root: &Path) -> miette::Result<Self> {
        debug!(
            worktree = ?worktree_root,
            "Loading worktree",
        );

        Ok(Self {
            git_dir: Self::load_git_file(worktree_root)?,
            type_of: GitTreeType::Worktree,
            work_dir: worktree_root.to_path_buf(),
            ..Default::default()
        })
    }

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
        let process = self.get_process();
        let mut args = vec![
            "diff",
            "--name-status",
            "--no-color",
            "--relative",
            // Ignore submodules since they would have different revisions
            "--ignore-submodules",
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
            let file = self.work_dir.join(line);
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
                'U' => {
                    statuses.push(ChangedStatus::Unstaged);
                }
                _ => {}
            }

            files.insert(file, statuses);
        }

        Ok(ChangedFiles { files })
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

        let paths = output
            .split('\n')
            .filter_map(|file| {
                // Paths are relative from the cwd
                let path = self.work_dir.join(file);

                // Do not include directories, which will be included in this list
                // when git encounters a submodule (it doesn't traverse into it)
                if path.is_file() { Some(path) } else { None }
            })
            .collect::<Vec<_>>();

        Ok(paths)
    }

    // https://git-scm.com/docs/git-ls-tree
    //
    // Requirements:
    //  Root/Worktree:
    //    - Run at the worktree root.
    //    - Includes submodule directories in the output, but not their files.
    //  Submodule:
    //    - Run in the submodule root.
    #[instrument(skip(self))]
    pub async fn exec_ls_tree(&self, revision: &str) -> miette::Result<BTreeMap<PathBuf, String>> {
        let process = self.get_process();
        let output = process
            .run_command(
                process.create_command_in_cwd(["ls-tree", "-r", "-z", revision], &self.work_dir),
                false,
            )
            .await?;

        let mut tree = BTreeMap::default();

        for line in output.split('\0') {
            if line.is_empty() {
                continue;
            }

            let parts = line.split(" ");

            if let Some((hash, path)) = parts.last().and_then(|part| part.split_once("\t")) {
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
        let mut args = vec!["merge-base".to_owned(), head_revision.to_owned()];
        let mut candidates = vec![base_revision.to_owned()];

        for remote in remote_candidates {
            candidates.push(format!("{remote}/{base_revision}"));
        }

        // To start, we need to find a working base
        let mut set = JoinSet::new();

        for candidate in candidates {
            let process = Arc::clone(self.process.as_ref().unwrap());
            let command = process
                .create_command_in_cwd(["merge-base", &candidate, head_revision], &self.work_dir);

            set.spawn(async move { process.run_command(command, true).await.map(|_| candidate) });
        }

        while let Some(result) = set.join_next().await {
            if let Ok(candidate) = result.into_diagnostic()? {
                args.push(candidate);
            }
        }

        // Then we need to run it again and extract the base hash.
        // This is necessary to support comparisons between forks!
        let process = self.get_process();

        if let Ok(hash) = process
            .run_command(process.create_command_in_cwd(args, &self.work_dir), true)
            .await
        {
            return Ok(Some(hash));
        }

        Ok(None)
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
            let file = self.work_dir.join(&line[3..]);
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
                'M' | 'R' => {
                    statuses.push(ChangedStatus::Modified);
                    statuses.push(ChangedStatus::Staged);
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
                'M' | 'R' => {
                    statuses.push(ChangedStatus::Modified);
                    statuses.push(ChangedStatus::Unstaged);
                }
                '?' => {
                    statuses.push(ChangedStatus::Untracked);
                }
                _ => {}
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
