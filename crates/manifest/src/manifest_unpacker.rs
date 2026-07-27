use crate::ManifestError;
use crate::manifest::{Manifest, ManifestFile};
use moon_blob::grant_owner_write_access;
use moon_common::path::{WorkspaceRelativePath, clean_components};
use starbase_utils::fs::{self, FsError};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ManifestUnpacker<'owner> {
    manifest: &'owner Manifest,
    workspace_root: PathBuf,
}

impl<'owner> ManifestUnpacker<'owner> {
    pub fn new(manifest: &'owner Manifest, workspace_root: PathBuf) -> Self {
        Self {
            manifest,
            workspace_root,
        }
    }

    pub fn unpack(self) -> miette::Result<()> {
        for file in &self.manifest.files {
            if file.digest.is_none() {
                continue;
            }

            let output_path = self.resolve_abs_path(&file.path)?;

            self.write_output_file(output_path, file)?;
        }

        for link in &self.manifest.symlinks {
            let output_path = self.resolve_abs_path(&link.path)?;

            self.link_output_file(
                self.resolve_abs_path(&link.target).map_err(|_| {
                    ManifestError::OutputSymlinkOutsideOfWorkspace {
                        output: output_path.clone(),
                        target: PathBuf::from(link.target.as_str()),
                    }
                })?,
                output_path,
            )?;
        }

        Ok(())
    }

    fn write_output_file(&self, output_path: PathBuf, file: &ManifestFile) -> miette::Result<()> {
        let map_error = |error| FsError::Write {
            path: output_path.clone(),
            error: Box::new(error),
        };

        // Reflink-or-copy from source file if available
        let fd = if let Some(source) = &file.source_path {
            fs::reflink_file(source, &output_path)?;

            // The reflink clones the source's permissions, which may lack the
            // write bit (stores populated before objects were normalized may
            // contain read-only blobs), so restore it before opening a handle
            // to apply the mtime/mode below
            grant_owner_write_access(&output_path)?;

            fs::open_file_for_writing(&output_path)?
        }
        // Otherwise write the bytes from the manifest
        else {
            let mut fd = fs::create_file(&output_path)?;

            fd.write_all(file.bytes.as_deref().unwrap_or_default())
                .map_err(map_error)?;

            fd
        };

        if let Some(modified) = &file.modified_at {
            fd.set_modified(*modified).map_err(map_error)?;
        }

        #[cfg(unix)]
        if let Some(mode) = &file.unix_mode {
            use std::os::unix::fs::PermissionsExt;

            fd.set_permissions(std::fs::Permissions::from_mode(*mode))
                .map_err(map_error)?;
        }

        Ok(())
    }

    // The manifest's unix mode is deliberately not applied: it records the
    // followed target's mode (which the target's own manifest entry restores),
    // and a chmod through the link would modify the target, not the link
    fn link_output_file(&self, from_path: PathBuf, to_path: PathBuf) -> miette::Result<()> {
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let map_error = |error| FsError::Create {
            path: to_path.clone(),
            error: Box::new(error),
        };

        #[cfg(windows)]
        {
            use std::os::windows::fs::{symlink_dir, symlink_file};

            if from_path.is_dir() {
                symlink_dir(&from_path, &to_path).map_err(map_error)?;
            } else {
                symlink_file(&from_path, &to_path).map_err(map_error)?;
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            symlink(&from_path, &to_path).map_err(map_error)?;
        }

        Ok(())
    }

    fn resolve_abs_path(&self, rel_path: &WorkspaceRelativePath) -> miette::Result<PathBuf> {
        let abs_path = Path::new(rel_path.as_str());

        if abs_path.is_absolute() {
            return Err(ManifestError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_path_buf(),
            }
            .into());
        }

        let output_path = clean_components(rel_path.to_logical_path(&self.workspace_root));

        if !output_path.starts_with(&self.workspace_root) {
            return Err(ManifestError::OutputFileOutsideOfWorkspace {
                output: output_path,
            }
            .into());
        }

        Ok(output_path)
    }
}
