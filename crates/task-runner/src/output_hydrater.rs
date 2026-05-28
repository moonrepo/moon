use crate::remote_compat::*;
use crate::run_state::TaskRunState;
use bazel_remote_apis::build::bazel::remote::execution::v2::ActionResult;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::color;
use moon_remote::{RemoteDigestExt, RemoteService};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarUnpacker;
use starbase_utils::fs;
use std::fmt::Debug;
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
    pub app_context: &'task Arc<AppContext>,
    pub task: &'task Arc<Task>,
}

impl OutputHydrater<'_> {
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
            // Hydrate files
            for file in &mut action_result.output_files {
                if let Some(digest) = file
                    .digest
                    .as_ref()
                    .and_then(|digest| digest.to_local_digest().ok())
                {
                    // Empty files have well-known content; don't hit the CAS
                    // for them (mirrors the stderr/stdout handling below, and
                    // avoids a hard failure when the empty blob isn't locally
                    // present — for example after a remote-only fetch).
                    if digest.size == 0 {
                        file.contents = vec![];
                    } else {
                        file.contents = app_context.cache_engine.cas.read_bytes(&digest.hash)?;
                    }
                }
            }

            // Hydrate stderr
            if let Some(digest) = action_result
                .stderr_digest
                .as_ref()
                .and_then(|digest| digest.to_local_digest().ok())
                && action_result.stderr_raw.is_empty()
                && digest.size > 0
            {
                action_result.stderr_raw = app_context.cache_engine.cas.read_bytes(&digest.hash)?;
            }

            // Hydrate stdout
            if let Some(digest) = action_result
                .stdout_digest
                .as_ref()
                .and_then(|digest| digest.to_local_digest().ok())
                && action_result.stdout_raw.is_empty()
                && digest.size > 0
            {
                action_result.stdout_raw = app_context.cache_engine.cas.read_bytes(&digest.hash)?;
            }

            Ok::<_, miette::Report>(action_result)
        })
        .await
        .into_diagnostic()??;

        // Write outputs to the project
        self.write_outputs(&action_result)?;

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
                hash, archive_file = ?archive_file, "Hydrating task outputs from local cache archive (legacy)"
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

    fn write_outputs(&self, result: &ActionResult) -> miette::Result<()> {
        for file in &result.output_files {
            if file.digest.is_some() {
                write_output_file(
                    self.app_context.workspace_root.join(&file.path),
                    &file.contents,
                    file,
                )?;
            }
        }

        // Create symlinks after output files have been written,
        // as the link target may reference one of these outputs
        for link in &result.output_symlinks {
            link_output_file(
                self.app_context.workspace_root.join(&link.target),
                self.app_context.workspace_root.join(&link.path),
                link,
            )?;
        }

        Ok(())
    }
}
