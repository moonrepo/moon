use crate::task_runner_error::TaskRunnerError;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project::Project;
use moon_remote::{ActionState, RemoteService};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarPacker;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, warn};

/// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    pub app: &'task AppContext,
    pub project: &'task Project,
    pub task: &'task Task,
}

impl OutputArchiver<'_> {
    #[instrument(skip(self, remote_state))]
    pub async fn archive(
        &self,
        hash: &str,
        remote_state: Option<&mut ActionState<'_>>,
    ) -> miette::Result<Option<PathBuf>> {
        let mut archived = false;
        let archive_file = self.app.cache_engine.hash.get_archive_path(hash);

        // Check that outputs actually exist
        if self.task.is_build_type() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        // If so, create and pack the archive!
        if archive_file.exists() {
            archived = true;
        } else if self.app.cache_engine.is_writable() {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Archiving task outputs from project"
            );

            self.create_local_archive(hash, &archive_file)?;
            archived = true;
        } else {
            debug!(
                task_target = self.task.target.as_str(),
                hash, "Cache is not writable, skipping output archiving"
            );
        }

        // Then cache the result in the remote service
        if let Some(state) = remote_state {
            archived = self.upload_to_remote_service(state).await?;
        }

        Ok(if archived { Some(archive_file) } else { None })
    }

    #[instrument(skip(self))]
    pub fn has_outputs_been_created(&self, bypass_globs: bool) -> miette::Result<bool> {
        let has_globs = !self.task.output_globs.is_empty();
        let all_negated_globs = self
            .task
            .output_globs
            .iter()
            .all(|glob| glob.as_str().starts_with('!'));

        // If using globs, we have no way to truly determine if all outputs
        // exist on the current file system, so always hydrate...
        if bypass_globs && has_globs && !all_negated_globs {
            return Ok(false);
        }

        // Check paths first since they are literal
        for output in &self.task.output_files {
            if !output.to_path(&self.app.workspace_root).exists() {
                return Ok(false);
            }
        }

        // Check globs last, as they are costly!
        // If all globs are negated, then the empty check will always
        // fail, resulting in archives not being created
        if has_globs && !all_negated_globs {
            let outputs = self
                .task
                .get_output_files(&self.app.workspace_root, false)?;

            return Ok(!outputs.is_empty());
        }

        Ok(true)
    }

    #[instrument(skip(self))]
    fn create_local_archive(&self, hash: &str, archive_file: &Path) -> miette::Result<()> {
        debug!(
            task_target = self.task.target.as_str(),
            hash,
            archive_file = ?archive_file, "Creating archive file"
        );

        // Create the archiver instance based on task outputs
        let mut archive = Archiver::new(&self.app.workspace_root, archive_file);

        for output_file in &self.task.output_files {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in &self.task.output_globs {
            archive.add_source_glob(output_glob.as_str());
        }

        // Also include stdout/stderr logs in the tarball
        let state_dir = self
            .app
            .cache_engine
            .state
            .get_target_dir(&self.task.target);

        archive.add_source_file(state_dir.join("stdout.log"), None);

        archive.add_source_file(state_dir.join("stderr.log"), None);

        // Pack the archive
        if let Err(error) = archive.pack(TarPacker::new_gz) {
            warn!(
                task_target = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Failed to package outputs into archive: {}",
                color::muted_light(error.to_string()),
            );
        }

        Ok(())
    }

    #[instrument(skip(self, state))]
    async fn upload_to_remote_service(&self, state: &mut ActionState<'_>) -> miette::Result<bool> {
        if let Some(remote) = RemoteService::session() {
            state.compute_outputs(&self.app.workspace_root)?;

            match remote.save_action(state).await {
                Ok(saved) => {
                    // Saves in a background thread
                    remote.save_action_result(state).await?;

                    return Ok(saved);
                }
                Err(error) => {
                    // If the task is successful but the upload fails,
                    // we don't want to mark the task as failed, so
                    // don't bubble up the error
                    warn!(
                        "Failed to upload to remote service: {}",
                        color::muted_light(error.to_string())
                    );
                }
            }
        }

        Ok(false)
    }
}
