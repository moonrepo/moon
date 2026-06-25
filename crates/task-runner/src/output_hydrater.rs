use crate::remote_compat::*;
use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use bazel_remote_apis::build::bazel::remote::execution::v2::ActionResult;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::{
    color,
    path::{PathExt, clean_components},
};
use moon_remote::{RemoteDigestExt, RemoteService};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarUnpacker;
use starbase_utils::{fs, glob::GlobSet};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};

#[derive(Clone, PartialEq)]
pub enum HydrateFrom {
    PreviousOutput,
    LocalArchive,
    LocalCache(ActionResult),
    RemoteCache(ActionResult),
}

impl Debug for HydrateFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HydrateFrom::PreviousOutput => write!(f, "PreviousOutput"),
            HydrateFrom::LocalArchive => write!(f, "LocalArchive"),
            HydrateFrom::LocalCache(_) => write!(f, "LocalCache"),
            HydrateFrom::RemoteCache(_) => write!(f, "RemoteCache"),
        }
    }
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
        from: &mut HydrateFrom,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<bool> {
        match from {
            HydrateFrom::PreviousOutput => Ok(true),

            HydrateFrom::LocalArchive => self.unpack_local_archive(hash, state).await,

            HydrateFrom::LocalCache(result) => self.hydrate_local(hash, state, result).await,

            HydrateFrom::RemoteCache(result) => self.hydrate_remote(hash, state, result).await,
        }
    }

    async fn hydrate_local(
        &self,
        hash: &str,
        state: &TaskRunState,
        result: &mut ActionResult,
    ) -> miette::Result<bool> {
        if !state.local_cache_readable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Local cache is not readable, skipping output hydration"
            );

            return Ok(false);
        }

        if !state.local_cas_enabled {
            return self.unpack_local_archive(hash, state).await;
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Hydrating task outputs from local cache"
        );

        // Fetch all outputs from the local CAS
        let app_context = Arc::clone(self.app_context);
        let mut action_result = result.clone();

        let action_result = spawn_blocking(move || {
            // Output files are materialized by reflinking them straight out of
            // the CAS (see `hydrate_outputs_from_cas` below), so their bytes are
            // never buffered here. Only the small inline logs are read into
            // memory, since the operation needs them to reconstruct stdio.

            // Hydrate stderr
            if let Some(digest) = action_result
                .stderr_digest
                .as_ref()
                .and_then(|digest| digest.to_local_digest().ok())
                && action_result.stderr_raw.is_empty()
                && digest.size > 0
            {
                action_result.stderr_raw = app_context.cache_engine.cas.read(&digest.hash)?;
            }

            // Hydrate stdout
            if let Some(digest) = action_result
                .stdout_digest
                .as_ref()
                .and_then(|digest| digest.to_local_digest().ok())
                && action_result.stdout_raw.is_empty()
                && digest.size > 0
            {
                action_result.stdout_raw = app_context.cache_engine.cas.read(&digest.hash)?;
            }

            Ok::<_, miette::Report>(action_result)
        })
        .await
        .into_diagnostic()??;

        // Materialize output files by reflinking them out of the local CAS — a
        // copy-on-write clone that avoids round-tripping each file's bytes
        // through memory the way `read` + write would.
        self.hydrate_outputs_from_cas(&action_result)?;

        *result = action_result;

        Ok(true)
    }

    async fn hydrate_remote(
        &self,
        hash: &str,
        state: &TaskRunState,
        result: &mut ActionResult,
    ) -> miette::Result<bool> {
        if !state.remote_cache_readable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Remote cache is not readable, attempting from local cache"
            );

            return self.hydrate_local(hash, state, result).await;
        };

        if state.digest.is_valid()
            && let Some(remote) = RemoteService::session()
        {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Hydrating task outputs from remote cache"
            );

            self.delete_existing_outputs()?;

            match remote.restore_action_result(&state.digest, result).await {
                Ok(restored) => {
                    if restored {
                        self.write_outputs(result)?;

                        return Ok(true);
                    } else {
                        self.delete_existing_outputs()?;
                    }
                }
                Err(error) => {
                    // If the download fails, we don't want to mark
                    // the task as cached and to re-run instead, so
                    // don't bubble up the error
                    warn!(
                        task_target = self.task.target.as_str(),
                        hash,
                        "Failed to download action result from remote service: {}",
                        color::muted_light(error.to_string())
                    );
                }
            }
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Failed to hydrate outputs from remote cache, attempting from local cache"
        );

        self.hydrate_local(hash, state, result).await
    }

    #[instrument(skip(self, state))]
    async fn unpack_local_archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<bool> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);

        if state.local_cache_readable && archive_file.exists() {
            debug!(
                task_target = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Hydrating task outputs from local cache archive (legacy)"
            );
        } else if !state.local_cache_readable || !archive_file.exists() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not readable, skipping output hydration"
            );

            return Ok(false);
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
        }

        Ok(true)
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

    /// Write outputs whose bytes are already in memory (the remote-cache path,
    /// where the download populates `file.contents`).
    fn write_outputs(&self, result: &ActionResult) -> miette::Result<()> {
        for file in &result.output_files {
            if file.digest.is_some() {
                let output_path = self.resolve_declared_output_path(&file.path)?;

                write_output_file(output_path, &file.contents, file)?;
            }
        }

        self.write_output_symlinks(result)
    }

    /// Restore outputs that live in the local CAS by reflinking each object
    /// directly to its declared path — a copy-on-write clone that never loads
    /// the file into memory. The local-cache hydration path.
    fn hydrate_outputs_from_cas(&self, result: &ActionResult) -> miette::Result<()> {
        let cas = &self.app_context.cache_engine.cas;

        for file in &result.output_files {
            let Some(digest) = file
                .digest
                .as_ref()
                .and_then(|digest| digest.to_local_digest().ok())
            else {
                continue;
            };

            // Resolve (and validate) the destination before touching disk, so
            // an untrusted action result can't escape the workspace.
            let output_path = self.resolve_declared_output_path(&file.path)?;

            if digest.size == 0 {
                // Empty files have well-known content, and the empty blob may
                // not be present locally (e.g. after a remote-only fetch), so
                // write it directly rather than reflinking from the CAS.
                write_output_file(output_path, b"", file)?;
            } else {
                cas.read_file(&digest.hash, &output_path)?;

                // The reflink clones content but not the original mtime/mode.
                apply_output_file_properties(&output_path, file)?;
            }
        }

        self.write_output_symlinks(result)
    }

    fn write_output_symlinks(&self, result: &ActionResult) -> miette::Result<()> {
        // Create symlinks after output files have been written,
        // as the link target may reference one of these outputs
        for link in &result.output_symlinks {
            let target_path = self.resolve_workspace_path(&link.target).map_err(|_| {
                TaskRunnerError::OutputSymlinkOutsideOfWorkspace {
                    output: PathBuf::from(&link.path),
                    target: PathBuf::from(&link.target),
                }
            })?;
            let link_path = self.resolve_declared_output_path(&link.path)?;

            link_output_file(target_path, link_path, link)?;
        }

        Ok(())
    }

    fn resolve_workspace_path(&self, raw_path: &str) -> miette::Result<PathBuf> {
        let raw_path = Path::new(raw_path);

        if raw_path.is_absolute() {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: raw_path.to_path_buf(),
            }
            .into());
        }

        let output_path = clean_components(self.app_context.workspace_root.join(raw_path));

        if !output_path.starts_with(&self.app_context.workspace_root) {
            return Err(TaskRunnerError::OutputFileOutsideOfWorkspace {
                output: output_path,
            }
            .into());
        }

        Ok(output_path)
    }

    fn resolve_declared_output_path(&self, raw_path: &str) -> miette::Result<PathBuf> {
        let output_path = self.resolve_workspace_path(raw_path)?;
        let rel_path = output_path
            .relative_to(&self.app_context.workspace_root)
            .into_diagnostic()?;

        if self.task.output_files.contains_key(&rel_path) {
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
