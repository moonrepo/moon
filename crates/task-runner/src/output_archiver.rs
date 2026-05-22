use crate::TaskRunState;
use crate::output_tree::{OutputDigestsMap, OutputTree};
use crate::remote_compat::{create_action, create_action_result};
use crate::task_runner_error::TaskRunnerError;
use miette::IntoDiagnostic;
use moon_app_context::AppContext;
use moon_common::color;
use moon_hash::Blob;
use moon_remote::RemoteService;
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
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
    pub async fn archive(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<OutputDigestsMap> {
        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        // Step 1) Collect all outputs
        let outputs = self.collect_output_blobs(false).await?;

        // Step 2) Extract the digests to store in state
        let digests = outputs.get_digests();

        // Step 3) Store in local and remote caches
        self.save_in_cas(hash, state, outputs).await?;

        // Step 4) Create the archive file (temporary)
        self.archive_legacy(hash, state).await?;

        // // Use the CAS if the experiment is enabled
        // if self
        //     .app_context
        //     .workspace_config
        //     .experiments
        //     .cas_outputs_cache
        // {
        //     return self.archive_modern(hash, state).await;
        // }

        // // Otherwise use the legacy archive file approach
        // self.archive_legacy(hash, state).await

        Ok(digests)
    }

    #[instrument(skip(self, state))]
    pub async fn archive_legacy(
        &self,
        hash: &str,
        state: &TaskRunState,
    ) -> miette::Result<OutputDigestsMap> {
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

        // Then cache the result in the remote service
        // if self.is_remote_cache_writable() {
        //     self.store_in_remote_cache(hash, state, self.collect_output_blobs(false).await?)
        //         .await?;
        // }

        Ok(OutputDigestsMap::default())
    }

    // #[instrument(skip(self, state))]
    // pub async fn archive_modern(
    //     &self,
    //     hash: &str,
    //     state: &TaskRunState,
    // ) -> miette::Result<OutputDigestsMap> {
    //     dbg!(&hash, &state);

    //     // Step 1) Save the outputs to local cache and gather blobs
    //     let outputs = self.store_in_local_cache(hash).await?;

    //     // Step 2) Extract the hashes to store in state
    //     let digests = outputs.get_digests();

    //     // Step 3) Upload these blobs to remote cache
    //     self.store_in_remote_cache(hash, state, outputs).await?;

    //     Ok(digests)
    // }

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
    async fn collect_output_blobs(&self, cas: bool) -> miette::Result<OutputTree> {
        let app_context = Arc::clone(self.app_context);
        let mut outputs = OutputTree::new(&app_context.workspace_root);
        let output_paths = self
            .task
            .get_output_files(&app_context.workspace_root, true)?;

        let tree = spawn_blocking(move || {
            // Read blobs
            for path in output_paths {
                outputs.insert(path, None)?;
            }

            // Store in CAS
            if cas {
                for blob in outputs.files.values() {
                    app_context.cache_engine.cas.write_blob(blob)?;
                }
            }

            Ok::<_, miette::Report>(outputs)
        })
        .await
        .into_diagnostic()??;

        Ok(tree)
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

    // #[instrument(skip(self))]
    // async fn store_in_local_cache(&self, hash: &str) -> miette::Result<OutputTree> {
    //     let store_local = self.is_local_cache_writable();
    //     let store_remote = self.is_remote_cache_writable();

    //     if store_local {
    //         debug!(
    //             task_target = self.task.target.as_str(),
    //             hash, "Storing task outputs in local cache"
    //         );
    //     } else if store_remote {
    //         debug!(
    //             task_target = self.task.target.as_str(),
    //             hash, "Local cache not enabled but extracting task outputs for remote cache"
    //         );
    //     } else {
    //         debug!(
    //             task_target = self.task.target.as_str(),
    //             hash, "Cache is not writable, skipping task output archiving"
    //         );
    //     }

    //     self.collect_output_blobs(store_local).await
    // }

    // #[instrument(skip(self, state))]
    // async fn store_in_remote_cache(
    //     &self,
    //     hash: &str,
    //     state: &TaskRunState,
    //     outputs: OutputTree,
    // ) -> miette::Result<bool> {
    //     if !self.is_remote_cache_writable() {
    //         return Ok(false);
    //     }

    //     let Some(remote) = RemoteService::session() else {
    //         return Ok(false);
    //     };

    //     debug!(
    //         task_target = self.task.target.as_str(),
    //         hash, "Storing task outputs in remote cache"
    //     );

    //     let action = create_action(&state.digest);

    //     match remote.save_action(action).await {
    //         Ok(digest) => {
    //             if let Some(digest) = &digest {
    //                 let (action_result, blobs) = create_action_result(&state.operation, outputs)?;

    //                 remote
    //                     .save_action_result(digest, action_result, blobs)
    //                     .await?;
    //             }

    //             Ok(digest.is_some())
    //         }
    //         Err(error) => {
    //             // If the task is successful but the upload fails,
    //             // we don't want to mark the task as failed, so
    //             // don't bubble up the error
    //             warn!(
    //                 "Failed to upload to remote service: {}",
    //                 color::muted_light(error.to_string())
    //             );

    //             Ok(false)
    //         }
    //     }
    // }

    #[instrument(skip_all)]
    async fn save_in_cas(
        &self,
        hash: &str,
        state: &TaskRunState,
        outputs: OutputTree,
    ) -> miette::Result<()> {
        let store_local = self.is_local_cache_writable();
        let store_remote = self.is_remote_cache_writable();
        let mut continue_remote = true;

        if store_local && store_remote {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in local and remote caches"
            );
        } else if store_local {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in local cache"
            );
        } else if store_remote {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Storing task outputs in remote cache"
            );
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping task output archiving"
            );

            return Ok(());
        }

        // Create and store the action first
        let action = create_action(&state.digest);
        let action_blob = Blob::from_data(&action)?;
        let action_digest = action_blob.digest.clone();

        if store_local {
            self.app_context.cache_engine.cas.write_blob(&action_blob)?;
        }

        if store_remote && let Some(remote) = RemoteService::session() {
            match remote.save_action(action, action_blob).await {
                Ok(saved) => {
                    continue_remote = saved;
                }
                Err(error) => {
                    warn!(
                        "Failed to upload action to remote service: {}",
                        color::muted_light(error.to_string())
                    );

                    continue_remote = false;
                }
            };
        }

        // Then create and store the action result
        let (action_result, blobs) = create_action_result(&state.operation, outputs)?;

        if store_local {
            for blob in &blobs {
                self.app_context.cache_engine.cas.write_blob(blob)?;
            }
        }

        if store_remote
            && continue_remote
            && let Some(remote) = RemoteService::session()
        {
            match remote
                .save_action_result(&action_digest, action_result, blobs)
                .await
            {
                Ok(_) => {}
                Err(error) => {
                    warn!(
                        "Failed to upload action result to remote service: {}",
                        color::muted_light(error.to_string())
                    );
                }
            };
        }

        Ok(())
    }
}
