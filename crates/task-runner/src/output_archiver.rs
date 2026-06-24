use crate::output_tree::OutputTree;
use crate::remote_compat::{create_action, create_action_blob, create_action_result};
use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_blob::Blob;
use moon_common::{BLOCKING_THREAD_COUNT, color};
use moon_hash::Digest;
use moon_remote::RemoteService;
use moon_task::Task;
use rustc_hash::FxHashSet;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::{JoinSet, spawn_blocking};
use tracing::{debug, instrument, warn};

/// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
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
    pub async fn archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<bool> {
        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        if state.local_cache_writable && state.remote_cache_writable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in local and remote caches"
            );
        } else if state.local_cache_writable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in local cache"
            );
        } else if state.remote_cache_writable {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in remote cache"
            );
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping task output archiving"
            );

            return Ok(false);
        }

        // Collect all outputs (streams each file directly into the CAS)
        let outputs = self.collect_output_blobs(hash).await?;
        let mut archived = !outputs.is_empty();

        // Store action + result in local and/or remote caches
        self.save_in_cas(hash, state, outputs).await?;

        // Create the archive file (temporary)
        if !state.local_cas_enabled {
            archived = self.pack_local_archive(hash, state).await?;
        }

        Ok(archived)
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

    #[instrument(skip(self))]
    async fn collect_output_blobs(&self, hash: &str) -> miette::Result<OutputTree> {
        debug!(
            task_target = self.task.target.as_str(),
            hash, "Collecting task output blobs"
        );

        self.batch_read_blobs_for_local(
            self.task
                .get_output_files(&self.app_context.workspace_root, true)?,
        )
        .await
    }

    #[instrument(skip(self, state))]
    async fn pack_local_archive(&self, hash: &str, state: &TaskRunState) -> miette::Result<bool> {
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

            return Ok(false);
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

        let archived = spawn_blocking(move || {
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

        Ok(archived)
    }

    #[instrument(skip(self, state, outputs))]
    async fn save_in_cas(
        &self,
        hash: &str,
        state: &TaskRunState,
        outputs: OutputTree,
    ) -> miette::Result<()> {
        if !state.digest.is_valid() {
            return Ok(());
        }

        let cache_engine = &self.app_context.cache_engine;
        let mut continue_remote = true;

        // Create and store the action first
        let action = create_action(&state.digest);
        let action_blob = create_action_blob(&state.digest, &state.bytes);

        if state.local_cache_writable & state.local_cas_enabled {
            cache_engine.cas.store_blob(&action_blob)?;
        }

        if state.remote_cache_writable
            && let Some(remote) = RemoteService::session()
        {
            match remote.save_action(action, action_blob).await {
                Ok(saved) => {
                    continue_remote = saved;
                }
                Err(error) => {
                    warn!(
                        task_target = self.task.target.as_str(),
                        hash,
                        "Failed to upload action to remote service: {}",
                        color::muted_light(error.to_string())
                    );

                    continue_remote = false;
                }
            };
        }

        // Then create the action result. Output file blobs are already in the
        // CAS thanks to streaming collection — `output_digests` references them.
        // `inline_blobs` carries the small in-memory ones (stderr/stdout).
        let (action_result, inline_blobs, output_digests) =
            create_action_result(&state.operation, outputs)?;

        if state.local_cache_writable & state.local_cas_enabled {
            // Action results are keyed by our internal task hash locally
            // (not the Bazel Remote API digest).
            let action_result_blob = Blob::from_data(&action_result)?;

            cache_engine
                .ac
                .write(&state.digest.hash, &action_result_blob.bytes)?;

            // Inline blobs (stderr/stdout) still need to be written to the CAS;
            // output file blobs were already streamed there during collection.
            for blob in &inline_blobs {
                cache_engine.cas.store_blob(blob)?;
            }
        }

        if state.remote_cache_writable
            && continue_remote
            && let Some(remote) = RemoteService::session()
        {
            let blobs = self
                .batch_read_blobs_for_remote(inline_blobs, output_digests)
                .await?;

            remote
                .save_action_result(&state.digest, action_result, blobs)
                .await?;
        }

        Ok(())
    }

    /// Collect outputs into an `OutputTree`, streaming each file's bytes
    /// directly into the local CAS as we hash it. After this returns, each
    /// digest in the tree refers to a blob already on disk in the CAS —
    /// callers can read bytes back via `cas.read_bytes(&digest.hash)` without
    /// re-touching the source file.
    async fn batch_read_blobs_for_local(
        &self,
        mut output_paths: Vec<PathBuf>,
    ) -> miette::Result<OutputTree> {
        let mut set = JoinSet::new();
        let chunk_size = output_paths.len() / BLOCKING_THREAD_COUNT;

        while !output_paths.is_empty() {
            let mut outputs = OutputTree::new(&self.app_context.workspace_root);
            let cache_engine = Arc::clone(&self.app_context.cache_engine);
            let chunk = output_paths
                .drain(0..chunk_size.max(1).min(output_paths.len()))
                .collect::<Vec<_>>();

            set.spawn_blocking(move || {
                for path in chunk {
                    outputs.insert(path, &cache_engine.cas)?;
                }

                Ok::<_, miette::Report>(outputs)
            });
        }

        let mut outputs = OutputTree::new(&self.app_context.workspace_root);

        while let Some(chunk) = set.join_next().await {
            let tree = chunk.into_diagnostic()??;

            outputs.files.extend(tree.files);
            outputs.symlinks.extend(tree.symlinks);
        }

        Ok(outputs)
    }

    /// Reconstruct the full `Vec<Blob>` needed by the remote upload API by
    /// loading output file bytes from the CAS in parallel and combining them
    /// with the already-in-memory inline blobs (stderr/stdout).
    ///
    /// `output_digests` may contain duplicates (multiple output files with
    /// identical content). We dedupe by hash before reading so each unique
    /// blob hits the CAS exactly once.
    async fn batch_read_blobs_for_remote(
        &self,
        mut blobs: Vec<Blob>,
        output_digests: Vec<Digest>,
    ) -> miette::Result<Vec<Blob>> {
        if output_digests.is_empty() {
            return Ok(blobs);
        }

        let mut unique_digests = FxHashSet::from_iter(output_digests)
            .into_iter()
            .collect::<Vec<_>>();

        let mut set = JoinSet::new();
        let chunk_size = unique_digests.len() / BLOCKING_THREAD_COUNT;

        while !unique_digests.is_empty() {
            let cache_engine = Arc::clone(&self.app_context.cache_engine);
            let chunk = unique_digests
                .drain(0..chunk_size.max(1).min(unique_digests.len()))
                .collect::<Vec<_>>();

            set.spawn_blocking(move || {
                let mut blobs = Vec::with_capacity(chunk.len());

                for digest in chunk {
                    let bytes = cache_engine.cas.read(&digest.hash)?;
                    blobs.push(Blob::new(digest, bytes));
                }

                Ok::<_, miette::Report>(blobs)
            });
        }

        while let Some(chunk) = set.join_next().await {
            blobs.extend(chunk.into_diagnostic()??);
        }

        Ok(blobs)
    }
}
