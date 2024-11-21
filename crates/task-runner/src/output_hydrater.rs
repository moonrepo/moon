use crate::run_state::read_stdlog_state_files;
use moon_action::Operation;
use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_common::color;
use moon_remote::{Digest, RemoteService};
use moon_task::Task;
use starbase_archive::tar::TarUnpacker;
use starbase_archive::Archiver;
use starbase_utils::fs;
use std::path::Path;
use tracing::{debug, instrument, warn};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HydrateFrom {
    LocalCache,
    Moonbase,
    PreviousOutput,
    RemoteCache,
}

pub struct OutputHydrater<'task> {
    pub app: &'task AppContext,
    pub task: &'task Task,
}

impl<'task> OutputHydrater<'task> {
    #[instrument(skip(self, operation))]
    pub async fn hydrate(
        &self,
        from: HydrateFrom,
        digest: &Digest,
        operation: &mut Operation,
    ) -> miette::Result<bool> {
        match from {
            // Only hydrate when the hash is different from the previous build,
            // as we can assume the outputs from the previous build still exist?
            HydrateFrom::PreviousOutput => Ok(true),

            // Based on the remote execution APIs
            HydrateFrom::RemoteCache => self.download_from_remote_service(digest, operation).await,

            // Otherwise write to local cache, then download archive from moonbase
            HydrateFrom::LocalCache | HydrateFrom::Moonbase => {
                let archive_file = self.app.cache_engine.hash.get_archive_path(&digest.hash);

                if self.app.cache_engine.is_readable() {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash = &digest.hash,
                        "Hydrating cached outputs into project"
                    );

                    // Attempt to download from remote cache to `.moon/outputs/<hash>`
                    if !archive_file.exists() && matches!(from, HydrateFrom::Moonbase) {
                        self.download_from_remote_storage(&digest.hash, &archive_file)
                            .await?;
                    }

                    // Otherwise hydrate the cached archive into the task's outputs
                    if archive_file.exists() {
                        self.unpack_local_archive(&digest.hash, &archive_file)?;

                        read_stdlog_state_files(
                            self.app
                                .cache_engine
                                .state
                                .get_target_dir(&self.task.target),
                            operation,
                        )?;

                        return Ok(true);
                    }
                } else {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash = &digest.hash,
                        "Cache is not readable, skipping output hydration"
                    );
                }

                Ok(false)
            }
        }
    }

    #[instrument(skip(self))]
    fn unpack_local_archive(&self, hash: &str, archive_file: &Path) -> miette::Result<bool> {
        debug!(
            task_target = self.task.target.as_str(),
            hash,
            archive_file = ?archive_file, "Unpacking archive into project"
        );

        // Create the archiver instance based on task outputs
        let mut archive = Archiver::new(&self.app.workspace_root, archive_file);

        for output_file in &self.task.output_files {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in &self.task.output_globs {
            archive.add_source_glob(output_glob.as_str());
        }

        // Unpack the archive
        if let Err(error) = archive.unpack(TarUnpacker::new_gz) {
            warn!(
                task_target = self.task.target.as_str(),
                hash,
                archive_file = ?archive_file,
                "Failed to hydrate outputs from archive: {}",
                color::muted_light(error.to_string()),
            );

            // Delete target outputs to ensure a clean slate
            for output in &self.task.output_files {
                fs::remove_file(output.to_logical_path(&self.app.workspace_root))?;
            }
        }

        Ok(true)
    }

    #[instrument(skip(self))]
    async fn download_from_remote_storage(
        &self,
        hash: &str,
        archive_file: &Path,
    ) -> miette::Result<()> {
        if let Some(moonbase) = Moonbase::session() {
            moonbase
                .download_artifact_from_remote_storage(hash, archive_file)
                .await?;
        }

        Ok(())
    }

    #[instrument(skip(self, operation))]
    async fn download_from_remote_service(
        &self,
        digest: &Digest,
        operation: &mut Operation,
    ) -> miette::Result<bool> {
        if let Some(remote) = RemoteService::session() {
            remote.restore_operation(digest, operation).await?;

            return Ok(true);
        }

        Ok(false)
    }
}
