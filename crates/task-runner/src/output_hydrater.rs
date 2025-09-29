use moon_app_context::AppContext;
use moon_common::color;
use moon_remote::{ActionState, RemoteService};
use moon_task::Task;
use starbase_archive::Archiver;
use starbase_archive::tar::TarUnpacker;
use starbase_utils::fs;
use std::path::Path;
use tracing::{debug, instrument, warn};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
    RemoteCache,
}

pub struct OutputHydrater<'task> {
    pub app_context: &'task AppContext,
    pub task: &'task Task,
}

impl OutputHydrater<'_> {
    #[instrument(skip(self, remote_state))]
    pub async fn hydrate(
        &self,
        from: HydrateFrom,
        hash: &str,
        remote_state: Option<&mut ActionState<'_>>,
    ) -> miette::Result<bool> {
        match from {
            // Only hydrate when the hash is different from the previous build,
            // as we can assume the outputs from the previous build still exist?
            HydrateFrom::PreviousOutput => Ok(true),

            // Based on the remote execution APIs
            HydrateFrom::RemoteCache => {
                if let Some(state) = remote_state
                    && self.task.options.cache.is_remote_enabled()
                {
                    self.download_from_remote_service(state).await
                } else {
                    Ok(false)
                }
            }

            // Otherwise write to local cache
            _ => {
                let archive_file = self.app_context.cache_engine.hash.get_archive_path(hash);
                let mut hydrated = false;

                if self.app_context.cache_engine.is_readable() {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash, "Hydrating cached outputs into project"
                    );

                    // Otherwise hydrate the cached archive into the task's outputs
                    if archive_file.exists() {
                        self.unpack_local_archive(hash, &archive_file)?;
                        hydrated = true
                    }
                } else {
                    debug!(
                        task_target = self.task.target.as_str(),
                        hash, "Cache is not readable, skipping output hydration"
                    );
                }

                Ok(hydrated)
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
        let mut archive = Archiver::new(&self.app_context.workspace_root, archive_file);

        for output_file in self.task.output_files.keys() {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in self.task.output_globs.keys() {
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

            self.delete_existing_outputs()?;
        }

        Ok(true)
    }

    #[instrument(skip(self, state))]
    async fn download_from_remote_service(
        &self,
        state: &mut ActionState<'_>,
    ) -> miette::Result<bool> {
        if let Some(remote) = RemoteService::session() {
            self.delete_existing_outputs()?;

            match remote.restore_action_result(state).await {
                Ok(restored) => {
                    if !restored {
                        self.delete_existing_outputs()?;
                    }

                    return Ok(restored);
                }
                Err(error) => {
                    // If the download fails, we don't want to mark
                    // the task as cached and to re-run instead, so
                    // don't bubble up the error
                    warn!(
                        "Failed to download from remote service: {}",
                        color::muted_light(error.to_string())
                    );
                }
            }
        }

        Ok(false)
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
}
