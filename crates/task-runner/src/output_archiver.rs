use crate::manifest_compat::ManifestBuilder;
use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_blob::{BlobContent, BlobInput};
use moon_cache::{Manifest, StorageOptions};
use moon_common::color;
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
use std::sync::Arc;
use tokio::task::spawn_blocking;
use tracing::{debug, instrument, warn};

pub enum ArchiveOutcome {
    Skipped,
    Queued,
}

/// Cache outputs to the `.moon/cache` folder and to cloud storage,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    app_context: &'task Arc<AppContext>,
    task: &'task Arc<Task>,
}

impl OutputArchiver<'_> {
    pub fn new<'task>(
        app_context: &'task Arc<AppContext>,
        task: &'task Arc<Task>,
    ) -> miette::Result<OutputArchiver<'task>> {
        Ok(OutputArchiver { task, app_context })
    }

    #[instrument(skip(self, state))]
    pub async fn archive(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<ArchiveOutcome> {
        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        let task_target = self.task.target.as_str();

        if state.local_cache_writable && state.remote_cache_writable {
            debug!(
                task_target,
                hash, "Storing task outputs in local and remote caches"
            );
        } else if state.local_cache_writable {
            debug!(task_target, hash, "Storing task outputs in local cache");
        } else if state.remote_cache_writable {
            debug!(task_target, hash, "Storing task outputs in remote cache");
        } else {
            debug!(
                task_target,
                hash, "Cache is not writable, skipping task output archiving"
            );

            return Ok(ArchiveOutcome::Skipped);
        }

        let use_local = state.local_cas_enabled && state.local_cache_writable;
        let use_remote = state.remote_cache_writable;

        // Store the manifest in the local/remote caches
        if use_local || use_remote {
            let manifest = self.create_cache_manifest(state).await?;

            self.app_context
                .cache_engine
                .storage
                .with_options(StorageOptions {
                    include_local: use_local,
                    include_remote: use_remote,
                    ..Default::default()
                })
                .archive_manifest(&state.digest, manifest, self.get_action_blob(state))
                .await?;
        }

        // Create the archive file (legacy / temporary)
        if !state.local_cas_enabled && state.local_cache_writable {
            self.pack_local_archive(hash, state).await?;
        }

        Ok(ArchiveOutcome::Queued)
    }

    /// The action digest addresses moon's fingerprint hash manifest at
    /// `.moon/cache/hashes/<hash>.json` — that file *is* the blob the digest
    /// names. Backends that validate the Bazel RE contract reject an action
    /// result whose action digest is absent from the CAS, so it must be
    /// uploaded alongside the outputs. Returns `None` when the file is absent
    /// (e.g. archiving without a computed fingerprint), leaving the upload a
    /// no-op rather than an error.
    fn get_action_blob(&self, state: &TaskRunState) -> Option<BlobInput> {
        let path = self
            .app_context
            .cache_engine
            .hash
            .get_manifest_path(&state.digest.hash);

        path.exists().then(|| BlobInput {
            content: BlobContent::File(path),
            digest: state.digest.clone(),
        })
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

    #[instrument(skip(self, state))]
    async fn create_cache_manifest(&self, state: &TaskRunState) -> miette::Result<Manifest> {
        let task = Arc::clone(self.task);
        let workspace_root = self.app_context.workspace_root.clone();

        // Building the manifest incurs a lot of file system calls,
        // so we run it in a blocking thread to avoid blocking the async runtime
        let mut builder = spawn_blocking(move || {
            let outputs = task.get_output_files(&workspace_root, true)?;
            let mut builder = ManifestBuilder::new(workspace_root);

            for output in outputs {
                builder.inherit_output(output)?;
            }

            Ok::<_, miette::Report>(builder)
        })
        .await
        .into_diagnostic()??;

        // Then inherit the execution operation metadata
        builder.inherit_operation(&state.operation)?;

        Ok(builder.build())
    }

    #[instrument(skip(self, state))]
    async fn pack_local_archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<()> {
        let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);

        if state.local_cache_writable && archive_file.exists() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Skipping archive, task outputs already persisted"
            );
        } else if !state.local_cache_writable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping output archiving"
            );

            return Ok(());
        }

        debug!(
            task_target = self.task.target.as_str(),
            hash,
            archive_file = ?archive_file, "Archiving task outputs from project"
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
                    "Failed to package task outputs into archive: {}",
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
}
