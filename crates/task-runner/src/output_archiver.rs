use crate::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::color;
use moon_common::path::PathExt;
use moon_hash::{Blob, OutputBlobs, OutputHashes};
use moon_remote::RemoteService;
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};

/// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    pub app_context: &'task Arc<AppContext>,
    pub task: &'task Arc<Task>,
}

impl OutputArchiver<'_> {
    #[instrument(skip(self, state))]
    pub async fn archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<OutputHashes> {
        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        // Use the CAS if the experiment is enabled
        if self
            .app_context
            .workspace_config
            .experiments
            .cas_outputs_cache
        {
            return self.archive_modern(hash, state).await;
        }

        // Otherwise use the legacy archive file approach
        self.archive_legacy(hash, state).await
    }

    #[instrument(skip(self, state))]
    pub async fn archive_legacy(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<OutputHashes> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);

        if self.app_context.cache_engine.is_readable() && archive_file.exists() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Skipping archive, task outputs already persisted"
            );
        } else if self.is_local_cache_writable() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Archiving task outputs from project"
            );

            self.create_local_archive(hash, archive_file).await?;
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping output archiving"
            );
        }

        // We need to always collect and extract outputs
        let blobs = self.collect_output_blobs(false).await?;
        let hashes = self.extract_output_hashes(&blobs)?;

        // Then cache the result in the remote service
        self.store_in_remote_cache(hash, state, blobs).await?;

        Ok(hashes)
    }

    #[instrument(skip(self, state))]
    pub async fn archive_modern(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<OutputHashes> {
        // Step 1) Save the outputs to local cache and gather blobs
        let blobs = self.store_in_local_cache(hash).await?;

        // Step 2) Extract the hashes to store in state
        let hashes = self.extract_output_hashes(&blobs)?;

        // Step 3) Upload these blobs to remote cache
        self.store_in_remote_cache(hash, state, blobs).await?;

        Ok(hashes)
    }

    #[instrument(skip(self))]
    pub fn has_outputs_been_created(&self, bypass_globs: bool) -> miette::Result<bool> {
        let has_globs = !self.task.output_globs.is_empty();
        let all_negated_globs = self
            .task
            .output_globs
            .keys()
            .all(|glob| glob.as_str().starts_with('!'));

        // If using globs, we have no way to truly determine if all outputs
        // exist on the current file system, so always hydrate...
        if bypass_globs && has_globs && !all_negated_globs {
            return Ok(false);
        }

        // Check paths first since they are literal
        for (output, params) in &self.task.output_files {
            if !output.to_path(&self.app_context.workspace_root).exists() && !params.optional {
                return Ok(false);
            }
        }

        // Check globs last, as they are costly!
        // If all globs are negated, then the empty check will always
        // fail, resulting in archives not being created
        if has_globs && !all_negated_globs {
            let outputs = self
                .task
                .get_output_files(&self.app_context.workspace_root, false)?;

            if outputs.is_empty()
                && !self
                    .task
                    .outputs
                    .iter()
                    .all(|output| output.is_glob() && output.is_optional())
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn is_local_cache_writable(&self) -> bool {
        self.app_context.cache_engine.is_writable() && self.task.options.cache.is_local_enabled()
    }

    fn is_remote_cache_writable(&self) -> bool {
        self.app_context.cache_engine.is_writable()
            && self.task.options.cache.is_remote_enabled()
            && RemoteService::is_enabled()
    }

    #[instrument(skip(self))]
    async fn collect_output_blobs(&self, cas: bool) -> miette::Result<OutputBlobs> {
        let app_context = Arc::clone(self.app_context);
        let output_paths = self
            .task
            .get_output_files(&app_context.workspace_root, true)?;

        let outputs = spawn_blocking(move || {
            let mut outputs = OutputBlobs::default();

            for path in output_paths {
                let blob = if cas {
                    app_context.cache_engine.cas.write_file(&path)?
                } else {
                    Blob::from_file(&path)?
                };

                outputs.insert(path, blob);
            }

            Ok::<_, miette::Report>(outputs)
        })
        .await
        .into_diagnostic()??;

        Ok(outputs)
    }

    fn extract_output_hashes(&self, outputs: &OutputBlobs) -> miette::Result<OutputHashes> {
        let mut hashes = BTreeMap::new();

        for (path, blob) in outputs {
            hashes.insert(
                path.relative_to(&self.app_context.workspace_root)
                    .into_diagnostic()?,
                blob.digest.hash.clone(),
            );
        }

        Ok(hashes)
    }

    #[instrument(skip(self))]
    async fn create_local_archive(&self, hash: &str, archive_file: PathBuf) -> miette::Result<()> {
        debug!(
            task_target = self.task.target.as_str(),
            hash,
            archive_file = ?archive_file, "Creating archive file"
        );

        // Clone values to run in a blocking thread
        let app_context = Arc::clone(self.app_context);
        let task = Arc::clone(self.task);
        let hash = hash.to_string();

        spawn_blocking(move || {
            // Create the archiver instance based on task outputs
            let mut archive = Archiver::new(&app_context.workspace_root, &archive_file);

            for output_file in task.output_files.keys() {
                archive.add_source_file(output_file.as_str(), None);
            }

            for output_glob in task.output_globs.keys() {
                archive.add_source_glob(output_glob.as_str());
            }

            // Also include stdout/stderr logs in the tarball
            let state_dir = app_context.cache_engine.state.get_target_dir(&task.target);

            archive.add_source_file(state_dir.join("stdout.log"), None);

            archive.add_source_file(state_dir.join("stderr.log"), None);

            // Pack the archive
            if let Err(error) = archive.pack(TarPacker::new_gz) {
                warn!(
                    task_target = task.target.as_str(),
                    hash,
                    archive_file = ?archive_file,
                    "Failed to package outputs into archive: {}",
                    color::muted_light(error.to_string()),
                );

                return false;
            }

            true
        })
        .await
        .into_diagnostic()?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn store_in_local_cache(&self, hash: &str) -> miette::Result<OutputBlobs> {
        let store_local = self.is_local_cache_writable();
        let store_remote = self.is_remote_cache_writable();

        if store_local {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in local cache"
            );
        } else if store_remote {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Local cache not enabled but extracting task outputs for remote cache"
            );
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping task output archiving"
            );
        }

        self.collect_output_blobs(store_local).await
    }

    #[instrument(skip(self, state, outputs))]
    async fn store_in_remote_cache(
        &self,
        hash: &str,
        state: &TaskRunState,
        outputs: OutputBlobs,
    ) -> miette::Result<bool> {
        if !self.is_remote_cache_writable() {
            return Ok(false);
        }

        let Some(remote) = RemoteService::session() else {
            return Ok(false);
        };

        debug!(
            task_target = self.task.target.as_str(),
            hash, "Storing task outputs in remote cache"
        );

        match remote
            .save_action(&state.action_digest, &state.action_bytes)
            .await
        {
            Ok(saved) => {
                remote
                    .save_action_result(&state.action_digest, &state.operation, outputs)
                    .await?;

                Ok(saved)
            }
            Err(error) => {
                // If the task is successful but the upload fails,
                // we don't want to mark the task as failed, so
                // don't bubble up the error
                warn!(
                    "Failed to upload to remote service: {}",
                    color::muted_light(error.to_string())
                );

                Ok(false)
            }
        }
    }
}
