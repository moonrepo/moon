use crate::errors::VcsError;
use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use cached::{CachedAsync, TimedCache};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use moon_config::VcsConfig;
use moon_error::MoonError;
use moon_utils::process::{output_to_string, output_to_trimmed_string, Command};
use moon_utils::{fs, string_vec};
use regex::Regex;
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

// Minimum version for APIs is v2.22!

pub struct Git {
    cache: Arc<RwLock<TimedCache<String, String>>>,
    config: VcsConfig,
    ignore: Option<Gitignore>,
    root: PathBuf,
}

impl Git {
    pub fn load(config: &VcsConfig, working_dir: &Path) -> VcsResult<Self> {
        let root = match fs::find_upwards(".git", working_dir) {
            Some(dir) => dir.parent().unwrap().to_path_buf(),
            None => working_dir.to_path_buf(),
        };

        let mut ignore: Option<Gitignore> = None;
        let ignore_path = root.join(".gitignore");

        if ignore_path.exists() {
            let mut builder = GitignoreBuilder::new(&root);

            if let Some(error) = builder.add(ignore_path) {
                return Err(VcsError::Ignore(error));
            }

            ignore = Some(builder.build().map_err(VcsError::Ignore)?);
        }

        Ok(Git {
            cache: Arc::new(RwLock::new(TimedCache::with_lifespan(15))),
            config: config.to_owned(),
            ignore,
            root,
        })
    }

    pub fn extract_slug_from_remote(output: String) -> Result<String, VcsError> {
        // git@github.com:moonrepo/moon.git
        let remote = if output.starts_with("git@") {
            format!("https://{}", output.replace(':', "/"))
            // https://github.com/moonrepo/moon
        } else {
            output
        };

        let url = url::Url::parse(&remote)
            .map_err(|e| VcsError::FailedToParseGitRemote(e.to_string()))?;
        let mut slug = url.path();

        if slug.starts_with('/') {
            slug = &slug[1..];
        }

        if slug.ends_with(".git") {
            slug = &slug[0..(slug.len() - 4)];
        }

        Ok(slug.to_owned())
    }

    async fn get_merge_base(&self, base: &str, head: &str) -> VcsResult<String> {
        let mut args = string_vec!["merge-base", head];
        let mut candidates = string_vec![base.to_owned()];

        for remote in &self.config.remote_candidates {
            candidates.push(format!("{remote}/{base}"));
        }

        // To start, we need to find a working base origin
        for candidate in candidates {
            if self
                .run_command(
                    self.create_command(vec!["merge-base", &candidate, head]),
                    true,
                )
                .await
                .is_ok()
            {
                args.push(candidate.clone());
            }
        }

        // Then we need to run it again and extract the base hash using the found origins
        // This is necessary to support comparisons between forks!
        if let Ok(hash) = self
            .run_command(
                self.create_command(args.iter().map(|a| a.as_str()).collect()),
                true,
            )
            .await
        {
            return Ok(hash);
        }

        Ok(base.to_owned())
    }

    fn is_file_ignored(&self, file: &str) -> bool {
        if self.ignore.is_some() {
            self.ignore
                .as_ref()
                .unwrap()
                .matched(file, false)
                .is_ignore()
        } else {
            false
        }
    }

    async fn run_command(&self, mut command: Command, trim: bool) -> VcsResult<String> {
        let mut cache = self.cache.write().await;
        let (mut cache_key, _) = command.get_command_line();

        cache_key += command.get_input_line().as_ref();

        if trim {
            cache_key += " [trimmed]";
        }

        let value: Result<_, MoonError> = cache
            .try_get_or_set_with(cache_key, || async {
                let output = command.exec_capture_output().await?;

                Ok(if trim {
                    output_to_trimmed_string(&output.stdout)
                } else {
                    output_to_string(&output.stdout)
                })
            })
            .await;

        Ok(value?.to_owned())
    }
}

#[async_trait]
impl Vcs for Git {
    fn create_command(&self, args: Vec<&str>) -> Command {
        let mut cmd = Command::new("git");
        cmd.args(args).cwd(&self.root);
        cmd
    }

    async fn get_local_branch(&self) -> VcsResult<String> {
        // --show-current was added in 2.22.0
        if let Ok(branch) = self
            .run_command(self.create_command(vec!["branch", "--show-current"]), true)
            .await
        {
            return Ok(branch);
        }

        self.run_command(
            self.create_command(vec!["rev-parse --abbrev-ref HEAD"]),
            true,
        )
        .await
    }

    async fn get_local_branch_revision(&self) -> VcsResult<String> {
        self.run_command(self.create_command(vec!["rev-parse", "HEAD"]), true)
            .await
    }

    fn get_default_branch(&self) -> &str {
        &self.config.default_branch
    }

    async fn get_default_branch_revision(&self) -> VcsResult<String> {
        self.run_command(
            self.create_command(vec!["rev-parse", &self.config.default_branch]),
            true,
        )
        .await
    }

    async fn get_file_hashes(
        &self,
        files: &[String],
        allow_ignored: bool,
    ) -> VcsResult<BTreeMap<String, String>> {
        let mut objects = vec![];
        let mut map = BTreeMap::new();

        for file in files {
            // File must exists or git fails
            if self.root.join(file).exists() && (allow_ignored || !self.is_file_ignored(file)) {
                objects.push(file.clone());
            }
        }

        if objects.is_empty() {
            return Ok(map);
        }

        // Sort for deterministic caching within the vcs layer
        objects.sort();

        let mut command = self.create_command(vec!["hash-object", "--stdin-paths"]);
        command.input(&[objects.join("\n")]);

        let output = self.run_command(command, true).await?;

        for (index, hash) in output.split('\n').enumerate() {
            if !hash.is_empty() {
                map.insert(objects[index].clone(), hash.to_owned());
            }
        }

        Ok(map)
    }

    async fn get_file_tree_hashes(&self, dir: &str) -> VcsResult<BTreeMap<String, String>> {
        // Extract all tracked and untracked files in the directory
        let output = self
            .run_command(
                self.create_command(vec![
                    "ls-files",
                    "--cached",
                    "--modified",
                    "--others",
                    "--full-name",
                    // Added in v2.31
                    // "--deduplicate",
                    "--exclude-standard",
                    dir,
                ]),
                true,
            )
            .await?;

        let files = output
            .split('\n')
            .map(|f| f.to_owned())
            .collect::<Vec<String>>();

        // Convert these file paths to hashes. We can't use `git ls-tree` as it
        // doesn't take untracked/modified files in the working tree into account.
        self.get_file_hashes(&files, false).await
    }

    async fn get_repository_slug(&self) -> VcsResult<String> {
        let output = self
            .run_command(
                self.create_command(vec!["remote", "get-url", "origin"]),
                true,
            )
            .await?;

        Self::extract_slug_from_remote(output)
    }

    // https://git-scm.com/docs/git-status#_short_format
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self
            .run_command(
                self.create_command(vec![
                    "status",
                    "--porcelain",
                    "--untracked-files",
                    // We use this option so that file names with special characters
                    // are displayed as-is and are not quoted/escaped
                    "-z",
                ]),
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
        let mut all = FxHashSet::default();
        let xy_regex = Regex::new(r"^(M|T|A|D|R|C|U|\?|!| )(M|T|A|D|R|C|U|\?|!| ) ").unwrap();

        // Lines are terminated by a NUL byte:
        //  XY file\0
        //  XY file\0orig_file\0
        for line in output.split('\0') {
            if line.is_empty() {
                continue;
            }

            // orig_file\0
            if !xy_regex.is_match(line) {
                continue;
            }

            // XY file\0
            let mut chars = line.chars();
            let x = chars.next().unwrap_or_default();
            let y = chars.next().unwrap_or_default();
            let file = String::from(&line[3..]);

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

            all.insert(file.clone());
        }

        Ok(TouchedFiles {
            added,
            all,
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
        let rev = if self.is_default_branch(revision) {
            "HEAD"
        } else {
            revision
        };

        Ok(self
            .get_touched_files_between_revisions(&format!("{rev}~1"), rev)
            .await?)
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> VcsResult<TouchedFiles> {
        let base = self.get_merge_base(base_revision, revision).await?;

        let output = self
            .run_command(
                self.create_command(vec![
                    "--no-pager",
                    "diff",
                    "--name-status",
                    "--no-color",
                    "--relative",
                    // We use this option so that file names with special characters
                    // are displayed as-is and are not quoted/escaped
                    "-z",
                    &base,
                ]),
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
        let mut all = FxHashSet::default();
        let x_with_score_regex = Regex::new(r"^(C|M|R)(\d{3})$").unwrap();
        let x_regex = Regex::new(r"^(A|D|M|T|U|X)$").unwrap();
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
            if x_with_score_regex.is_match(line) || x_regex.is_match(line) {
                last_status = &line[0..1];
                continue;
            }

            let x = last_status.chars().next().unwrap_or_default();
            let file = line.to_owned();

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

            all.insert(file.clone());
        }

        Ok(TouchedFiles {
            added,
            all,
            deleted,
            modified,
            staged,
            unstaged,
            untracked: FxHashSet::default(),
        })
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        let default_branch = &self.config.default_branch;

        if default_branch == branch {
            return true;
        }

        if default_branch.contains('/') {
            return default_branch.ends_with(&format!("/{branch}"));
        }

        false
    }

    fn is_enabled(&self) -> bool {
        self.root.join(".git").exists()
    }
}
