use crate::task_runner_error::TaskRunnerError;
use moon_config::ProjectConfig;
use moon_task::{TargetError, TargetScope, Task};
use moon_workspace::Workspace;
use starbase_archive::tar::TarPacker;
use starbase_archive::Archiver;
use starbase_utils::glob;
use std::path::{Path, PathBuf};

/// Cache outputs to the `.moon/cache/outputs` folder and to the cloud,
/// so that subsequent builds are faster, and any local outputs
/// can be hydrated easily.
pub struct OutputArchiver<'task> {
    pub project_config: &'task ProjectConfig,
    pub task: &'task Task,
    pub workspace: &'task Workspace,
}

impl<'task> OutputArchiver<'task> {
    pub async fn archive(&self, hash: &str) -> miette::Result<Option<PathBuf>> {
        if hash.is_empty() || !self.is_archivable()? {
            return Ok(None);
        }

        // Check that outputs actually exist
        if !self.task.outputs.is_empty() && !self.has_outputs_been_created(false)? {
            return Err(TaskRunnerError::MissingOutputs {
                target: self.task.target.to_string(),
            }
            .into());
        }

        // If so, create and pack the archive!
        let archive_file = self.workspace.cache_engine.hash.get_archive_path(hash);

        if self.workspace.cache_engine.get_mode().is_writable() && !archive_file.exists() {
            self.create_local_archive(&archive_file)?;

            if archive_file.exists() {
                self.upload_to_remote_storage(&archive_file, hash).await?;
            }
        }

        Ok(Some(archive_file))
    }

    pub fn is_archivable(&self) -> miette::Result<bool> {
        let task = self.task;

        if task.is_build_type() {
            return Ok(true);
        }

        for target in &self.workspace.config.runner.archivable_targets {
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
                    if self.project_config.tags.contains(tag_id) && is_matching_task {
                        return Ok(true);
                    }
                }
                TargetScope::Deps => return Err(TargetError::NoDepsInRunContext.into()),
                TargetScope::OwnSelf => return Err(TargetError::NoSelfInRunContext.into()),
            };
        }

        Ok(false)
    }

    pub fn has_outputs_been_created(&self, bypass_globs: bool) -> miette::Result<bool> {
        // If using globs, we have no way to truly determine if all outputs
        // exist on the current file system, so always hydrate...
        if bypass_globs && !self.task.output_globs.is_empty() {
            return Ok(false);
        }

        // Check paths first since they are literal
        for output in &self.task.output_files {
            if !output.to_path(&self.workspace.root).exists() {
                return Ok(false);
            }
        }

        // Check globs last, as they are costly
        if !self.task.output_globs.is_empty() {
            let outputs = glob::walk_files(&self.workspace.root, &self.task.output_globs)?;

            return Ok(!outputs.is_empty());
        }

        Ok(true)
    }

    pub fn create_local_archive(&self, archive_file: &Path) -> miette::Result<()> {
        // Create the archiver instance based on task outputs
        let mut archive = Archiver::new(&self.workspace.root, &archive_file);

        for output_file in &self.task.output_files {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in &self.task.output_globs {
            archive.add_source_glob(output_glob.as_str());
        }

        // Also include stdout/stderr logs at the root of the tarball
        let state_dir = self
            .workspace
            .cache_engine
            .state
            .get_target_dir(&self.task.target);

        archive.add_source_file(state_dir.join("stdout.log"), None);

        archive.add_source_file(state_dir.join("stderr.log"), None);

        archive.pack(TarPacker::new_gz)?;

        Ok(())
    }

    pub async fn upload_to_remote_storage(
        &self,
        archive_file: &Path,
        hash: &str,
    ) -> miette::Result<()> {
        if let Some(moonbase) = &self.workspace.session {
            if let Err(error) = moonbase
                .upload_artifact_to_remote_storage(hash, archive_file, &self.task.target.id)
                .await
            {
                // TODO log error
            }
        }

        Ok(())
    }
}
