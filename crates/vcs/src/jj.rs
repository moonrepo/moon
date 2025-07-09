use crate::process_cache::ProcessCache;
use crate::touched_files::TouchedFiles;
use crate::vcs::Vcs;
use async_trait::async_trait;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use miette::Diagnostic;
use moon_common::path::{RelativePath, RelativePathBuf, WorkspaceRelativePath, WorkspaceRelativePathBuf};
use moon_common::{Style, Stylize};
use semver::Version;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug, Diagnostic)]
pub enum JujutsuError {
    #[diagnostic(code(jj::invalid_version))]
    #[error("Invalid or unsupported jj version.")]
    InvalidVersion {
        #[source]
        error: Box<semver::Error>,
    },

    #[diagnostic(code(jj::ignore::load_invalid))]
    #[error("Failed to load and parse {}.", ".jjignore".style(Style::File))]
    JjignoreLoadFailed {
        #[source]
        error: Box<ignore::Error>,
    },

    #[diagnostic(code(jj::repository::extract_slug))]
    #[error("Failed to extract a repository slug from jj remote.")]
    ExtractRepoSlugFailed,

    #[diagnostic(code(jj::workspace::invalid))]
    #[error("Failed to detect jj workspace.")]
    InvalidWorkspace,
}

#[derive(Debug)]
pub struct Jujutsu {
    /// Ignore rules derived from a root `.jjignore` file.
    ignore: Option<Gitignore>,

    /// Default git branch name (for Git backend compatibility).
    pub default_branch: Arc<String>,

    /// Root of the `.jj` directory.
    pub jj_root: PathBuf,

    /// Run and cache `jj` commands.
    pub process: ProcessCache,

    /// List of remotes to use as merge candidates.
    pub remote_candidates: Vec<String>,

    /// Root of the repository that contains `.jj`.
    pub repository_root: PathBuf,

    /// Path between the jj and workspace root.
    pub root_prefix: Option<RelativePathBuf>,

    /// Current workspace name.
    pub workspace_name: Option<String>,
}

/// Represents a single workspace in a Jujutsu repository.
#[derive(Debug, Clone, Deserialize)]
pub struct JjWorkspace {
    pub name: String,
    pub path: PathBuf,
}

impl Jujutsu {
    pub fn load<R: AsRef<Path>, B: AsRef<str>>(
        workspace_root: R,
        default_branch: B,
        remote_candidates: &[String],
    ) -> miette::Result<Jujutsu> {
        debug!("Using jujutsu as a version control system");

        let workspace_root = workspace_root.as_ref();
        let default_branch = default_branch.as_ref();

        debug!(
            starting_dir = ?workspace_root,
            "Attempting to find a .jj directory"
        );

        // Find the .jj dir
        let mut current_dir = workspace_root;
        let repository_root;
        let jj_root;
        let mut workspace_name = None;

        loop {
            let jj_check = current_dir.join(".jj");

            if jj_check.exists() && jj_check.is_dir() {
                debug!(
                    jj = ?jj_check,
                    "Found a .jj directory (repository root)"
                );

                jj_root = jj_check.to_path_buf();
                repository_root = current_dir.to_path_buf();

                // Check if we're in a workspace
                if workspace_root != current_dir {
                    // Extract workspace name from path
                    if let Ok(rel_path) = workspace_root.strip_prefix(&repository_root) {
                        workspace_name = rel_path
                            .components()
                            .next()
                            .and_then(|c| c.as_os_str().to_str())
                            .map(|s| s.to_string());
                    }
                }

                break;
            }

            match current_dir.parent() {
                Some(parent) => current_dir = parent,
                None => {
                    debug!("Unable to find .jj, falling back to workspace root");

                    jj_root = workspace_root.join(".jj");
                    repository_root = workspace_root.to_path_buf();
                    break;
                }
            };
        }

        // Load .jjignore
        let ignore_path = repository_root.join(".jjignore");
        let mut ignore: Option<Gitignore> = None;

        if ignore_path.exists() {
            debug!(
                ignore_file = ?ignore_path,
                "Loading ignore rules from .jjignore",
            );

            let mut builder = GitignoreBuilder::new(&repository_root);

            if let Some(error) = builder.add(ignore_path) {
                return Err(JujutsuError::JjignoreLoadFailed {
                    error: Box::new(error),
                }
                .into());
            }

            // Also check for .gitignore if using Git backend
            let gitignore_path = repository_root.join(".gitignore");
            if gitignore_path.exists() {
                debug!(
                    ignore_file = ?gitignore_path,
                    "Loading ignore rules from .gitignore",
                );
                let _ = builder.add(gitignore_path);
            }

            ignore = Some(
                builder
                    .build()
                    .map_err(|error| JujutsuError::JjignoreLoadFailed {
                        error: Box::new(error),
                    })?,
            );
        }

        let jj = Jujutsu {
            default_branch: Arc::new(default_branch.to_owned()),
            ignore,
            remote_candidates: remote_candidates.to_owned(),
            root_prefix: if repository_root == workspace_root {
                None
            } else {
                RelativePathBuf::from_path(workspace_root.strip_prefix(&repository_root).unwrap())
                    .ok()
            },
            repository_root,
            process: ProcessCache::new("jj", workspace_root),
            jj_root,
            workspace_name,
        };

        Ok(jj)
    }

    /// Get the current change ID (Jujutsu's equivalent of a commit).
    async fn get_current_change_id(&self) -> miette::Result<Arc<String>> {
        self.process
            .run(["log", "--no-graph", "-r", "@", "-T", "change_id"], true)
            .await
    }

    /// Get the current commit ID.
    async fn get_current_commit_id(&self) -> miette::Result<Arc<String>> {
        self.process
            .run(["log", "--no-graph", "-r", "@", "-T", "commit_id"], true)
            .await
    }

    /// List all workspaces in the repository.
    pub async fn list_workspaces(&self) -> miette::Result<Vec<JjWorkspace>> {
        let output = self.process
            .run(["workspace", "list"], true)
            .await?;

        let mut workspaces = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                workspaces.push(JjWorkspace {
                    name: parts[0].to_string(),
                    path: PathBuf::from(parts[1]),
                });
            }
        }

        Ok(workspaces)
    }

    /// Parse file status from jj status output.
    fn parse_status_output(output: &str, root_prefix: &Option<RelativePathBuf>) -> TouchedFiles {
        let mut touched = TouchedFiles::default();
        let mut in_working_copy = false;

        for line in output.lines() {
            if line.starts_with("Working copy changes:") {
                in_working_copy = true;
                continue;
            }

            if !in_working_copy {
                continue;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let (status, file_path) = if let Some((status, path)) = trimmed.split_once(' ') {
                (status.trim(), path.trim())
            } else {
                continue;
            };

            // Convert path to workspace relative
            let mut path = WorkspaceRelativePathBuf::from(file_path);
            if let Some(prefix) = root_prefix {
                path = WorkspaceRelativePathBuf::from(prefix.join(&path));
            }

            match status {
                "A" => { touched.added.insert(path); },
                "D" => { touched.deleted.insert(path); },
                "M" => { touched.modified.insert(path); },
                _ => {}
            }
        }

        // In Jujutsu, all changes are automatically part of the working copy commit
        // so we don't have staged/unstaged distinction
        touched.staged = touched.added.clone();
        touched.staged.extend(touched.modified.clone());
        touched.staged.extend(touched.deleted.clone());

        touched
    }
}

#[async_trait]
impl Vcs for Jujutsu {
    async fn get_local_branch(&self) -> miette::Result<Arc<String>> {
        // Jujutsu doesn't have traditional branches, return the change ID
        self.get_current_change_id().await
    }

    async fn get_local_branch_revision(&self) -> miette::Result<Arc<String>> {
        self.get_current_commit_id().await
    }

    async fn get_default_branch(&self) -> miette::Result<Arc<String>> {
        // For Git backend compatibility, use the configured default branch
        Ok(Arc::clone(&self.default_branch))
    }

    async fn get_default_branch_revision(&self) -> miette::Result<Arc<String>> {
        // Get the commit ID of the default branch
        self.process
            .run(
                ["log", "--no-graph", "-r", &self.default_branch, "-T", "commit_id"],
                true,
            )
            .await
    }

    async fn get_file_hashes(
        &self,
        files: &[WorkspaceRelativePathBuf],
        allow_ignored: bool,
    ) -> miette::Result<BTreeMap<WorkspaceRelativePathBuf, String>> {
        let mut hashes = BTreeMap::new();

        for file in files {
            if !allow_ignored && self.is_ignored(Path::new(file.as_str())) {
                continue;
            }

            let file_path = if let Some(prefix) = &self.root_prefix {
                prefix.join(file).to_string()
            } else {
                file.to_string()
            };

            // Use jj cat to get file content and hash it
            if let Ok(content) = self.process.run(["cat", "-r", "@", &file_path], true).await {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(content.as_bytes());
                let hash = format!("{:x}", hasher.finalize());
                hashes.insert(file.clone(), hash);
            }
        }

        Ok(hashes)
    }

    async fn get_file_tree(
        &self,
        dir: &WorkspaceRelativePath,
    ) -> miette::Result<Vec<WorkspaceRelativePathBuf>> {
        let dir_path = if let Some(prefix) = &self.root_prefix {
            prefix.join(dir).to_string()
        } else {
            dir.to_string()
        };

        let output = self
            .process
            .run(["file", "list", "-r", "@", &dir_path], true)
            .await?;

        let mut files = Vec::new();
        for line in output.lines() {
            if !line.is_empty() {
                let mut path = WorkspaceRelativePathBuf::from(line);
                
                // Remove root prefix if present
                if let Some(prefix) = &self.root_prefix {
                    if let Ok(stripped) = RelativePath::new(&path).strip_prefix(prefix) {
                        path = WorkspaceRelativePathBuf::from(stripped);
                    }
                }
                
                files.push(path);
            }
        }

        Ok(files)
    }

    async fn get_hooks_dir(&self) -> miette::Result<PathBuf> {
        // Jujutsu doesn't have a hooks directory like Git
        // Return a path within .jj for compatibility
        Ok(self.jj_root.join("hooks"))
    }

    async fn get_repository_root(&self) -> miette::Result<PathBuf> {
        Ok(self.repository_root.clone())
    }

    async fn get_repository_slug(&self) -> miette::Result<Arc<String>> {
        // Try to get the Git remote URL if using Git backend
        for remote in &self.remote_candidates {
            if let Ok(url) = self
                .process
                .run(["git", "remote", "get-url", remote], true)
                .await
            {
                // Parse the URL to extract owner/repo
                let url = url.trim();
                
                // Handle SSH URLs
                if let Some(slug) = url
                    .strip_prefix("git@")
                    .and_then(|s| s.split_once(':'))
                    .and_then(|(_, path)| path.strip_suffix(".git"))
                {
                    return Ok(Arc::new(slug.to_string()));
                }
                
                // Handle HTTPS URLs
                if let Some(slug) = url
                    .strip_prefix("https://")
                    .and_then(|s| s.split_once('/'))
                    .map(|(_, path)| path.trim_end_matches(".git"))
                {
                    return Ok(Arc::new(slug.to_string()));
                }
            }
        }

        Err(JujutsuError::ExtractRepoSlugFailed.into())
    }

    async fn get_touched_files(&self) -> miette::Result<TouchedFiles> {
        let output = self.process.run(["status"], true).await?;
        Ok(Self::parse_status_output(&output, &self.root_prefix))
    }

    async fn get_touched_files_against_previous_revision(
        &self,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        // Get diff between revision and its parent
        let output = self
            .process
            .run(["diff", "-r", &format!("{revision}-"), "--summary"], true)
            .await?;

        Ok(Self::parse_status_output(&output, &self.root_prefix))
    }

    async fn get_touched_files_between_revisions(
        &self,
        base_revision: &str,
        revision: &str,
    ) -> miette::Result<TouchedFiles> {
        let output = self
            .process
            .run(
                ["diff", "-r", &format!("{base_revision}..{revision}"), "--summary"],
                true,
            )
            .await?;

        Ok(Self::parse_status_output(&output, &self.root_prefix))
    }

    async fn get_version(&self) -> miette::Result<Version> {
        let output = self.process.run(["--version"], false).await?;

        let version_str = output
            .split_whitespace()
            .nth(2)
            .unwrap_or("0.0.0")
            .trim_start_matches('v');

        Version::parse(version_str).map_err(|error| JujutsuError::InvalidVersion {
            error: Box::new(error),
        }.into())
    }

    async fn get_working_root(&self) -> miette::Result<PathBuf> {
        if let Some(name) = &self.workspace_name {
            // In a workspace, return the workspace root
            Ok(self.repository_root.join(name))
        } else {
            // In the main workspace
            Ok(self.repository_root.clone())
        }
    }

    fn is_default_branch(&self, branch: &str) -> bool {
        branch == self.default_branch.as_str()
    }

    fn is_enabled(&self) -> bool {
        self.jj_root.exists()
    }

    fn is_ignored(&self, file: &Path) -> bool {
        if let Some(ignore) = &self.ignore {
            let full_path = if file.is_absolute() {
                file.to_path_buf()
            } else if let Some(prefix) = &self.root_prefix {
                self.repository_root.join(prefix.to_string()).join(file)
            } else {
                self.repository_root.join(file)
            };

            ignore.matched(&full_path, full_path.is_dir()).is_ignore()
        } else {
            false
        }
    }

    async fn is_shallow_checkout(&self) -> miette::Result<bool> {
        // Jujutsu doesn't have shallow checkouts in the same way as Git
        Ok(false)
    }
}