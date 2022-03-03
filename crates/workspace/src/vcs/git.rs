use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use moon_utils::process::{create_command, exec_command_capture_stdout};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Git {
    default_branch: String,
    working_dir: PathBuf,
}

impl Git {
    pub fn new(default_branch: &str, working_dir: &Path) -> Self {
        Git {
            default_branch: String::from(default_branch),
            working_dir: working_dir.to_path_buf(),
        }
    }

    fn process_touched_files<F: Fn(String) -> (char, char, String)>(
        output: String,
        extract: F,
    ) -> TouchedFiles {
        if output.is_empty() {
            return TouchedFiles::default();
        }

        let mut added = HashSet::new();
        let mut deleted = HashSet::new();
        let mut modified = HashSet::new();
        let mut untracked = HashSet::new();
        let mut staged = HashSet::new();
        let mut unstaged = HashSet::new();
        let mut all = HashSet::new();
        let spaces_regex = Regex::new(r"\s+").unwrap();

        for line in output.split('\n') {
            if line.is_empty() {
                continue;
            }

            let clean_line = String::from(spaces_regex.replace_all(line, " "));
            let (x, y, file) = extract(clean_line);

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

        TouchedFiles {
            added,
            all,
            deleted,
            modified,
            staged,
            unstaged,
            untracked,
        }
    }
}

#[async_trait]
impl Vcs for Git {
    async fn get_local_branch(&self) -> VcsResult<String> {
        self.run_command(vec!["branch", "--show-current"], true)
            .await
    }

    async fn get_local_branch_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", "HEAD"], true).await
    }

    fn get_default_branch(&self) -> &str {
        &self.default_branch
    }

    async fn get_default_branch_hash(&self) -> VcsResult<String> {
        self.run_command(vec!["rev-parse", &self.default_branch], true)
            .await
    }

    // https://git-scm.com/docs/git-status#_short_format
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self
            .run_command(
                vec!["-c", "color.status=false", "status", "-s", "-u"],
                false,
            )
            .await?;

        // XY file/path [-> copied/file]
        Ok(Git::process_touched_files(output, |line| {
            let mut chars = line.chars();
            let x = chars.next().unwrap_or_default();
            let y = chars.next().unwrap_or_default();
            let mut file = &line[3..];

            if let Some(index) = file.find("->") {
                file = &file[index + 1..];
            }

            (x, y, file.to_owned())
        }))
    }

    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> VcsResult<TouchedFiles> {
        Ok(self
            .get_touched_files_between_revisions(&format!("{}~1", revision), revision)
            .await?)
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> VcsResult<TouchedFiles> {
        let output = self
            .run_command(
                vec![
                    "--no-pager",
                    "diff",
                    "--name-status",
                    "--no-color",
                    "--relative",
                    base_revision,
                    revision,
                ],
                false,
            )
            .await?;

        // X file/path [copied/file]
        Ok(Git::process_touched_files(output, |line| {
            let parts = line.split(' ').collect::<Vec<&str>>();
            let status = parts[0].chars().next().unwrap();
            let mut file = parts[1];

            if let Some(copied_file) = parts.get(2) {
                file = copied_file;
            }

            (status as char, ' ', file.to_owned())
        }))
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        if self.default_branch == branch {
            return true;
        }

        if self.default_branch.contains('/') {
            return self.default_branch.ends_with(&format!("/{}", branch));
        }

        false
    }

    async fn run_command(&self, args: Vec<&str>, trim: bool) -> VcsResult<String> {
        let output = exec_command_capture_stdout(
            create_command("git")
                .args(args)
                .current_dir(&self.working_dir),
        )
        .await?;

        if trim {
            return Ok(output.trim().to_owned());
        }

        Ok(output)
    }
}
