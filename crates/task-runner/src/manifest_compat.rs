use crate::task_runner_error::TaskRunnerError;
use moon_action::Operation;
use moon_blob::Blob;
use moon_cache::{Manifest, ManifestFile, ManifestSymlink};
use moon_common::path::{PathExt, WorkspaceRelativePathBuf};
use moon_hash::Digest;
use starbase_utils::fs::{self, FsError};
use starbase_utils::glob::{self, GlobWalkOptions};
use std::fs::Metadata;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ManifestBuilder {
    manifest: Manifest,
    workspace_root: PathBuf,
}

impl ManifestBuilder {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            manifest: Manifest::default(),
            workspace_root,
        }
    }

    pub fn build(self) -> Manifest {
        self.manifest
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
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace { output: abs_path }.into());
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
        for abs_file in
            glob::walk_fast_with_options(abs_path, ["**/*"], GlobWalkOptions::default().files())?
        {
            self.insert_file(abs_file)?;
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
            path: self.convert_path(&abs_path)?,
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

        if !link.starts_with(&self.workspace_root) {
            return Err(TaskRunnerError::OutputSymlinkOutsideOfWorkspace {
                output: abs_path,
                target: link,
            }
            .into());
        }

        let metadata = fs::metadata(&abs_path)?;

        self.manifest.symlinks.push(ManifestSymlink {
            modified_at: metadata.modified().ok(),
            path: self.convert_path(&abs_path)?,
            target: self.convert_path(&link)?,
            unix_mode: extract_unix_mode(&metadata),
        });

        Ok(())
    }

    fn convert_path(&self, abs_path: &Path) -> miette::Result<WorkspaceRelativePathBuf> {
        let rel_path = abs_path.relative_to(&self.workspace_root).map_err(|_| {
            TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_owned(),
            }
        })?;

        Ok(rel_path)
    }
}

#[cfg(unix)]
fn is_file_executable(_path: &Path, metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_file_executable(path: &Path, _metadata: &Metadata) -> bool {
    path.extension().is_some_and(|ext| ext == "exe")
}

#[cfg(unix)]
fn extract_unix_mode(metadata: &Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;

    Some(metadata.permissions().mode())
}

#[cfg(windows)]
fn extract_unix_mode(_metadata: &Metadata) -> Option<u32> {
    None
}
