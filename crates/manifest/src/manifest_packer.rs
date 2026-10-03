use crate::helpers::*;
use crate::manifest::{Manifest, ManifestFile, ManifestSymlink};
use crate::manifest_error::ManifestError;
use moon_action::Operation;
use moon_blob::Blob;
use moon_common::path::{PathExt, WorkspaceRelativePathBuf, clean_components};
use moon_hash::Digest;
use starbase_utils::fs::{self, FsError};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ManifestPacker {
    manifest: Manifest,
    workspace_root: PathBuf,
}

impl ManifestPacker {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            manifest: Manifest::default(),
            workspace_root,
        }
    }

    pub fn pack(self) -> Manifest {
        self.manifest
    }

    pub fn inherit_source(&mut self, digest: &Digest, path: PathBuf) -> miette::Result<()> {
        if path.exists() {
            self.manifest.digest_source = Some(ManifestFile {
                digest: Some(digest.to_owned()),
                path: self.resolve_rel_path(&path)?,
                source_path: Some(path),
                ..Default::default()
            });
        }

        Ok(())
    }

    pub fn inherit_operation(&mut self, operation: &Operation) -> miette::Result<()> {
        if let Some(exec) = operation.get_exec_output() {
            self.manifest.exit_code = exec.exit_code.unwrap_or_default();

            if let Some(stderr) = &exec.stderr {
                let blob = Blob::from_bytes(stderr.as_bytes().to_owned())?;

                self.manifest.stderr_digest = Some(blob.digest);
                self.manifest.stderr_bytes = Some(blob.bytes);
            }

            if let Some(stdout) = &exec.stdout {
                let blob = Blob::from_bytes(stdout.as_bytes().to_owned())?;

                self.manifest.stdout_digest = Some(blob.digest);
                self.manifest.stdout_bytes = Some(blob.bytes);
            }
        }

        Ok(())
    }

    pub fn inherit_output(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        if !abs_path.starts_with(&self.workspace_root) {
            return Err(ManifestError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
        }

        if abs_path.is_symlink() {
            self.insert_symlink(abs_path)?;
        } else if abs_path.is_file() {
            self.insert_file(abs_path)?;
        } else if abs_path.is_dir() {
            self.insert_dir(abs_path)?;
        }

        Ok(())
    }

    fn insert_dir(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        // Read the tree rather than glob it: a glob set applies the global
        // negations (`node_modules/**`, `.git`), which exist for source
        // globbing and would drop those subtrees from a task's own output.
        for entry in fs::read_dir_all(abs_path)? {
            // Entries are typed from the directory read, which doesn't follow
            // links, so a symlink arrives as a symlink. Hashing one as a file
            // follows it, which fails outright for a link to a directory.
            if entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_symlink())
            {
                self.insert_symlink(entry.path())?;
            } else {
                self.insert_file(entry.path())?;
            }
        }

        Ok(())
    }

    fn insert_file(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        let metadata = fs::metadata(&abs_path)?;

        self.manifest.files.push(ManifestFile {
            bytes: None,
            digest: Some(Digest::from_file(&abs_path)?),
            is_executable: is_file_executable(&abs_path, &metadata),
            modified_at: metadata.modified().ok(),
            path: self.resolve_rel_path(&abs_path)?,
            source_path: Some(abs_path),
            unix_mode: extract_unix_mode(&metadata),
        });

        Ok(())
    }

    fn insert_symlink(&mut self, abs_path: PathBuf) -> miette::Result<()> {
        let link = std::fs::read_link(&abs_path).map_err(|error| FsError::Read {
            path: abs_path.clone(),
            error: Box::new(error),
        })?;

        // A link target is stored verbatim, so a relative one resolves from the
        // link's own directory, not the workspace root. Resolve before testing
        // containment, otherwise every relative link looks external.
        let target = if link.is_absolute() {
            link.clone()
        } else {
            clean_components(
                abs_path
                    .parent()
                    .unwrap_or(self.workspace_root.as_path())
                    .join(&link),
            )
        };

        if !target.starts_with(&self.workspace_root) {
            return Err(ManifestError::OutputSymlinkOutsideOfWorkspace {
                output: abs_path,
                target: link,
            }
            .into());
        }

        let metadata = fs::metadata(&abs_path)?;

        self.manifest.symlinks.push(ManifestSymlink {
            modified_at: metadata.modified().ok(),
            path: self.resolve_rel_path(&abs_path)?,
            target: self.resolve_rel_path(&target)?,
            unix_mode: extract_unix_mode(&metadata),
        });

        Ok(())
    }

    fn resolve_rel_path(&self, abs_path: &Path) -> miette::Result<WorkspaceRelativePathBuf> {
        let rel_path = abs_path.relative_to(&self.workspace_root).map_err(|_| {
            ManifestError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_owned(),
            }
        })?;

        Ok(rel_path)
    }
}
