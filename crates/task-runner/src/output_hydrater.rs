use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_cache::{
    Manifest, ManifestFile, ManifestSource, StorageOptions, grant_owner_write_access,
};
use moon_common::{
    color,
    path::{WorkspaceRelativePath, clean_components},
};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarUnpacker;
use starbase_utils::{
    fs::{self, FsError},
    glob::GlobSet,
};
use std::fmt::{self, Debug};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};

pub enum HydrateFrom {
    PreviousOutput,
    LocalArchive,
    Storage(Box<ManifestSource>),
}

impl Debug for HydrateFrom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HydrateFrom::PreviousOutput => write!(f, "PreviousOutput"),
            HydrateFrom::LocalArchive => write!(f, "LocalArchive"),
            HydrateFrom::Storage(source) => write!(f, "Storage({})", source.backend.get_id()),
        }
    }
}

pub enum HydrateOutcome {
    Skipped,
    Missed,
    Hit,
    HitFromStorage(Manifest, bool),
}

pub struct OutputHydrater<'task> {
    app_context: &'task Arc<AppContext>,
    task: &'task Arc<Task>,
    task_output_globset: GlobSet<'static>,
}

impl OutputHydrater<'_> {
    pub fn new<'task>(
        app_context: &'task Arc<AppContext>,
        task: &'task Arc<Task>,
    ) -> miette::Result<OutputHydrater<'task>> {
        Ok(OutputHydrater {
            task_output_globset: GlobSet::new_owned(task.output_globs.keys())?,
            task,
            app_context,
        })
    }

    #[instrument(skip(self, state))]
    pub async fn hydrate(
        &self,
        from: HydrateFrom,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<HydrateOutcome> {
        match from {
            HydrateFrom::PreviousOutput => Ok(HydrateOutcome::Hit),

            HydrateFrom::LocalArchive => self.unpack_local_archive(hash, state).await,

            HydrateFrom::Storage(source) => {
                if !source.remote && !state.local_cas_enabled {
                    return self.unpack_local_archive(hash, state).await;
                }

                let task_target = self.task.target.as_str();

                if state.local_cache_readable && state.remote_cache_readable {
                    debug!(
                        task_target,
                        hash, "Hydrating task outputs from local and remote caches"
                    );
                } else if state.local_cache_readable {
                    debug!(task_target, hash, "Hydrating task outputs from local cache");
                } else if state.remote_cache_readable {
                    debug!(
                        task_target,
                        hash, "Hydrating task outputs from remote cache"
                    );
                } else {
                    debug!(
                        task_target,
                        hash, "Cache is not readable, skipping task output hydration"
                    );

                    return Ok(HydrateOutcome::Skipped);
                }

                let use_local = state.local_cas_enabled && state.local_cache_readable;
                let use_remote = state.remote_cache_readable;
                let is_remote_backend = source.remote;

                // Delete existing outputs first so that reflinking works
                self.delete_existing_outputs()?;

                // Retrieve the manifest from the local/remote caches
                let manifest = self
                    .app_context
                    .cache_engine
                    .storage
                    .with_options(StorageOptions {
                        include_local: use_local,
                        include_remote: use_remote,
                        ..Default::default()
                    })
                    .hydrate_manifest(&state.digest, *source)
                    .await?;

                if let Some(manifest) = &manifest {
                    self.write_manifest_outputs(manifest)?;
                }

                Ok(match manifest {
                    Some(manifest) => HydrateOutcome::HitFromStorage(manifest, is_remote_backend),
                    None => HydrateOutcome::Missed,
                })
            }
        }
    }

    #[instrument(skip(self))]
    fn write_manifest_outputs(&self, manifest: &Manifest) -> miette::Result<()> {
        for file in &manifest.files {
            if file.digest.is_none() {
                continue;
            }

            let output_path = self.resolve_declared_output_path(&file.path)?;

            self.write_output_file(output_path, file)?;
        }

        for link in &manifest.symlinks {
            let output_path = self.resolve_declared_output_path(&link.path)?;

            self.link_output_file(
                self.resolve_workspace_path(&link.target).map_err(|_| {
                    TaskRunnerError::OutputSymlinkOutsideOfWorkspace {
                        output: output_path.clone(),
                        target: PathBuf::from(link.target.as_str()),
                    }
                })?,
                output_path,
            )?;
        }

        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn unpack_local_archive(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<HydrateOutcome> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);
        let task_target = self.task.target.as_str();

        if state.local_cache_readable && archive_file.exists() {
            debug!(
                task_target,
                hash,
                archive_file = ?archive_file,
                "Hydrating task outputs from local cache archive (legacy)"
            );
        } else if !state.local_cache_readable || !archive_file.exists() {
            debug!(
                task_target,
                hash, "Cache is not readable, skipping output hydration"
            );

            return Ok(HydrateOutcome::Skipped);
        }

        // Clone values to run in a blocking thread
        let app_context = Arc::clone(self.app_context);
        let task = Arc::clone(self.task);
        let hash = hash.to_string();

        // Create the archiver instance based on task outputs
        let hydrated = spawn_blocking(move || {
            let mut archive = Archiver::new(&app_context.workspace_root, &archive_file);

            for output_file in task.output_files.keys() {
                archive.add_source_file(output_file.as_str(), None);
            }

            for output_glob in task.output_globs.keys() {
                archive.add_source_glob(output_glob.as_str());
            }

            // Unpack the archive
            if let Err(error) = archive.unpack(TarUnpacker::new_gz) {
                warn!(
                    task_target = task.target.as_str(),
                    hash,
                    archive_file = ?archive_file,
                    "Failed to hydrate task outputs from archive: {}",
                    color::muted_light(error.to_string()),
                );

                return false;
            }

            true
        })
        .await
        .into_diagnostic()?;

        if !hydrated {
            self.delete_existing_outputs()?;

            return Ok(HydrateOutcome::Missed);
        }

        Ok(HydrateOutcome::Hit)
    }

    fn delete_existing_outputs(&self) -> miette::Result<()> {
        for output in self
            .task
            .get_output_files(&self.app_context.workspace_root, true)?
        {
            // Ignore failures as we don't want to crash the entire pipeline,
            // and in most cases, these artifacts will just be overwritten
            // on the next hydration anyways!
            let _ = fs::remove(output);
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

    fn resolve_workspace_path(&self, rel_path: &WorkspaceRelativePath) -> miette::Result<PathBuf> {
        let abs_path = Path::new(rel_path.as_str());

        if abs_path.is_absolute() {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: abs_path.to_path_buf(),
            }
            .into());
        }

        let output_path =
            clean_components(rel_path.to_logical_path(&self.app_context.workspace_root));

        if !output_path.starts_with(&self.app_context.workspace_root) {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: output_path,
            }
            .into());
        }

        Ok(output_path)
    }

    fn resolve_declared_output_path(
        &self,
        rel_path: &WorkspaceRelativePath,
    ) -> miette::Result<PathBuf> {
        let output_path = self.resolve_workspace_path(rel_path)?;

        if self.task.output_files.contains_key(rel_path) {
            return Ok(output_path);
        }

        for declared_output in self.task.output_files.keys() {
            if rel_path.starts_with(declared_output) {
                return Ok(output_path);
            }
        }

        if !self.task.output_globs.is_empty() && self.task_output_globset.matches(rel_path.as_str())
        {
            return Ok(output_path);
        }

        Err(TaskRunnerError::OutputFileNotDeclared {
            target: self.task.target.clone(),
            output: output_path,
        }
        .into())
    }
}
