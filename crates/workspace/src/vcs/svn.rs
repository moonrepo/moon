use crate::vcs::{TouchedFiles, Vcs, VcsResult};
use async_trait::async_trait;
use moon_utils::process::{create_command, exec_command_capture_stdout};
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// TODO: This code hasn't been tested yet and may not be accurate!

pub struct Svn {
    default_branch: String,
    working_dir: PathBuf,
}

impl Svn {
    pub fn new(default_branch: &str, working_dir: &Path) -> Self {
        Svn {
            default_branch: String::from(default_branch),
            working_dir: working_dir.to_path_buf(),
        }
    }

    fn extract_line_from_info(&self, label: &str, info: &str) -> String {
        for line in info.split('\n') {
            if line.starts_with(label) {
                return String::from(line).replace(label, "").trim().to_owned();
            }
        }

        String::new()
    }

    async fn get_hash_for_rev(&self, rev: &str) -> VcsResult<String> {
        let output = self.run_command(vec!["info", "-r", rev], true).await?;

        Ok(self.extract_line_from_info("Revision:", &output))
    }
}

// https://edoras.sdsu.edu/doc/svn-book-html-chunk/svn.ref.svn.c.info.html
#[async_trait]
impl Vcs for Svn {
    async fn get_local_branch(&self) -> VcsResult<String> {
        let output = self.run_command(vec!["info"], false).await?;
        let url = self.extract_line_from_info("URL:", &output);
        let pattern = Regex::new("branches/([^/]+)").unwrap();

        if pattern.is_match(&url) {
            let caps = pattern.captures(&url).unwrap();

            return Ok(String::from(
                caps.get(1)
                    .map_or(self.default_branch.as_str(), |m| m.as_str()),
            ));
        }

        Ok(self.get_default_branch().to_owned())
    }

    async fn get_local_branch_hash(&self) -> VcsResult<String> {
        Ok(self.get_hash_for_rev("BASE").await?)
    }

    fn get_default_branch(&self) -> &str {
        &self.default_branch
    }

    async fn get_default_branch_hash(&self) -> VcsResult<String> {
        Ok(self.get_hash_for_rev("HEAD").await?)
    }

    // https://svnbook.red-bean.com/en/1.8/svn.ref.svn.c.status.html
    async fn get_touched_files(&self) -> VcsResult<TouchedFiles> {
        let output = self.run_command(vec!["status", "wc"], false).await?;

        if output.is_empty() {
            return Ok(TouchedFiles::default());
        }

        let mut added = HashSet::new();
        let mut deleted = HashSet::new();
        let mut modified = HashSet::new();
        let mut untracked = HashSet::new();
        let mut staged = HashSet::new();
        let unstaged = HashSet::new();
        let mut all = HashSet::new();

        for line in output.split('\n') {
            let mut chars = line.chars();
            let c1 = chars.next().unwrap_or_default();
            let c2 = chars.next().unwrap_or_default();
            let file = String::from(&line[8..]);

            match c1 {
                'A' | 'C' => {
                    added.insert(file.clone());
                }
                'D' => {
                    deleted.insert(file.clone());
                }
                'M' | 'R' => {
                    modified.insert(file.clone());
                    staged.insert(file.clone());
                }
                '?' => {
                    untracked.insert(file.clone());
                }
                _ => {}
            }

            if c2 == 'M' {
                modified.insert(file.clone());
            }

            all.insert(file.clone());

            // svn files are always staged by default
            staged.insert(file.clone());
        }

        Ok(TouchedFiles {
            added,
            all,
            deleted,
            modified,
            staged,
            unstaged, // svn has no concept for this
            untracked,
        })
    }

    // https://svnbook.red-bean.com/en/1.8/svn.ref.svn.c.status.html
    async fn get_touched_files_against_branch(&self, branch: &str) -> VcsResult<TouchedFiles> {
        Ok(TouchedFiles::default())
    }

    async fn run_command(&self, args: Vec<&str>, trim: bool) -> VcsResult<String> {
        let output = exec_command_capture_stdout(
            create_command("svn")
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
