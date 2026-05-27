use crate::output_tree::OutputTree;
use crate::remote_compat::{create_action, create_action_blob, create_action_result};
use crate::run_state::TaskRunState;
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::color;
use moon_hash::Blob;
use moon_remote::{RemoteService, partition_into_groups};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::task::{JoinSet, spawn_blocking};
use tracing::{debug, instrument, trace, warn};

/// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    pub app_context: &'task Arc<AppContext>,
    pub task: &'task Arc<Task>,
}

impl OutputArchiver<'_> {
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

        // Step 1) Collect all outputs
        let outputs = self.collect_output_blobs(hash).await?;
        let archived = !outputs.is_empty();

        // Step 2) Store in local and remote caches
        self.save_in_cas(hash, state, outputs).await?;

        // Step 3) Create the archive file (temporary)
        if !state.local_cas_enabled {
            self.pack_local_archive(hash, state).await?;
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
        trace!(
            task_target = self.task.target.as_str(),
            hash, "Collecting task output blobs"
        );

        self.batch_read_blobs(
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
            cache_engine.cas.write_blob(&action_blob)?;
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

        // Then create and store the action result
        let (action_result, mut blobs) = create_action_result(&state.operation, outputs)?;

        if state.local_cache_writable & state.local_cas_enabled {
            // Locally the action results are stored using our internal task hash,
            // and not the digest/hash that the Bazel Remote API expects
            let action_result_blob = Blob::from_data(&action_result)?;

            cache_engine
                .ac
                .write(&state.digest.hash, &action_result_blob.bytes)?;

            // However the blobs themselves are stored using their content hash
            blobs = self.batch_write_blobs(blobs).await?;
        }

        if state.remote_cache_writable
            && continue_remote
            && let Some(remote) = RemoteService::session()
        {
            remote
                .save_action_result(&state.digest, action_result, blobs)
                .await?;
        }

        Ok(())
    }

    async fn batch_read_blobs(&self, mut paths: Vec<PathBuf>) -> miette::Result<OutputTree> {
        let mut set = JoinSet::new();

        while !paths.is_empty() {
            let chunk = paths.drain(0..25.min(paths.len())).collect::<Vec<_>>();
            let mut outputs = OutputTree::new(&self.app_context.workspace_root);

            set.spawn_blocking(move || {
                for path in chunk {
                    outputs.insert(path, None)?;
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

    async fn batch_write_blobs(&self, blobs: Vec<Blob>) -> miette::Result<Vec<Blob>> {
        let mut set = JoinSet::new();

        // 2mb per thread
        for group in partition_into_groups(blobs, 2097152, |blob| blob.bytes.len()).into_values() {
            let cache_engine = Arc::clone(&self.app_context.cache_engine);

            set.spawn_blocking(move || {
                for blob in &group.items {
                    cache_engine.cas.write_blob(blob)?;
                }

                Ok::<_, miette::Report>(group.items)
            });
        }

        let mut blobs = vec![];

        while let Some(chunk) = set.join_next().await {
            blobs.extend(chunk.into_diagnostic()??);
        }

        Ok(blobs)
    }
}
