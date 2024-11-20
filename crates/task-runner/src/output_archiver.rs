use crate::task_runner_error::TaskRunnerError;
use moon_action::Operation;
use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_common::color;
use moon_project::Project;
use moon_remote::{compute_digests_for_outputs, Digest, RemoteService};
use moon_task::{TargetError, TargetScope, Task};
use starbase_archive::tar::TarPacker;
use starbase_archive::Archiver;
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

impl<'task> OutputArchiver<'task> {
    #[instrument(skip(self))]
    pub async fn archive(
        &self,
        digest: &Digest,
        operation: Option<&Operation>,
    ) -> miette::Result<Option<PathBuf>> {
        if !self.is_archivable()? {
            return Ok(None);
        }

        // Check that outputs actually exist
        if !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.clone(),
            }
            .into());
        }

        // If so, create and pack the archive!
        let archive_file = self.app.cache_engine.hash.get_archive_path(&digest.hash);

        if !archive_file.exists() {
            if self.app.cache_engine.is_writable() {
                debug!(
                    task_target = self.task.target.as_str(),
                    hash = &digest.hash,
                    "Archiving task outputs from project"
                );

                self.create_local_archive(&digest.hash, &archive_file)?;

                if archive_file.exists() {
                    self.upload_to_remote_storage(&digest.hash, &archive_file)
                        .await?;
                }
            } else {
                debug!(
                    task_target = self.task.target.as_str(),
                    hash = &digest.hash,
                    "Cache is not writable, skipping output archiving"
                );
            }
        }

        // Then cache the result in the remote service
        if let Some(operation) = operation {
            self.upload_to_remote_service(digest, operation).await?;
        }

        Ok(Some(archive_file))
    }

    pub fn is_archivable(&self) -> miette::Result<bool> {
        let task = self.task;

        if task.is_build_type() {
            return Ok(true);
        }

        for target in &self.app.workspace_config.runner.archivable_targets {
            let is_matching_task = task.target.task_id == target.task_id;

            match &target.scope {
                TargetScope::All => {
                    if is_matching_task {
                        return Ok(true);
                    }
                }
                TargetScope::Project(project_locator) => {
                    if let Some(owner_id) = task.target.get_project_id() {
                        if owner_id == project_locator && is_matching_task {
                            return Ok(true);
                        }
                    }
                }
                TargetScope::Tag(tag_id) => {
                    if self.project.config.tags.contains(tag_id) && is_matching_task {
                        return Ok(true);
                    }
                }
                TargetScope::Deps => return Err(TargetError::NoDepsInRunContext.into()),
                TargetScope::OwnSelf => return Err(TargetError::NoSelfInRunContext.into()),
            };
        }

        Ok(false)
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

    #[instrument(skip(self))]
    async fn upload_to_remote_storage(
        &self,
        hash: &str,
        archive_file: &Path,
    ) -> miette::Result<()> {
        if let Some(moonbase) = Moonbase::session() {
            moonbase
                .upload_artifact_to_remote_storage(hash, archive_file, &self.task.target.id)
                .await?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn upload_to_remote_service(
        &self,
        digest: &Digest,
        operation: &Operation,
    ) -> miette::Result<()> {
        if let Some(remote) = RemoteService::session() {
            let output_digests = compute_digests_for_outputs(
                self.task.get_output_files(&self.app.workspace_root, true)?,
                &self.app.workspace_root,
            )?;

            remote
                .save_operation_with_outputs(digest, operation, output_digests)
                .await?;
        }

        Ok(())
    }
}
