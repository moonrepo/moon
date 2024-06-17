use moon_api::Moonbase;
use moon_app_context::AppContext;
use moon_common::color;
use moon_task::Task;
use starbase_archive::tar::TarUnpacker;
use starbase_archive::Archiver;
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
    pub app: &'task AppContext,
    pub task: &'task Task,
}

impl<'task> OutputHydrater<'task> {
    #[instrument(skip(self))]
    pub async fn hydrate(&self, hash: &str, from: HydrateFrom) -> miette::Result<bool> {
        // Only hydrate when the hash is different from the previous build,
        // as we can assume the outputs from the previous build still exist?
        if matches!(from, HydrateFrom::PreviousOutput) {
            return Ok(true);
        }

        let archive_file = self.app.cache_engine.hash.get_archive_path(hash);

        if self.app.cache_engine.is_readable() {
            debug!(
                task = self.task.target.as_str(),
                hash, "Hydrating cached outputs into project"
            );

            // Attempt to download from remote cache to `.moon/outputs/<hash>`
            if !archive_file.exists() && matches!(from, HydrateFrom::RemoteCache) {
                self.download_from_remote_storage(hash, &archive_file)
                    .await?;
            }

            // Otherwise hydrate the cached archive into the task's outputs
            if archive_file.exists() {
                self.unpack_local_archive(hash, &archive_file)?;

                return Ok(true);
            }
        } else {
            debug!(
                task = self.task.target.as_str(),
                hash, "Cache is not readable, skipping output hydration"
            );
        }

        Ok(false)
    }

    #[instrument(skip(self))]
    pub fn unpack_local_archive(&self, hash: &str, archive_file: &Path) -> miette::Result<bool> {
        debug!(
            task = self.task.target.as_str(),
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
                task = self.task.target.as_str(),
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
    pub async fn download_from_remote_storage(
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
}
