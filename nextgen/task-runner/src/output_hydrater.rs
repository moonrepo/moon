use moon_cache::CacheEngine;
use moon_common::color;
use moon_task::Task;
use starbase_archive::tar::TarUnpacker;
use starbase_archive::Archiver;
use starbase_utils::fs;
use std::path::Path;
use tracing::warn;

#[derive(Clone, Copy)]
pub enum HydrateFrom {
    LocalCache,
    PreviousOutput,
    RemoteCache,
}

pub struct OutputHydrater<'task> {
    pub cache_engine: &'task CacheEngine,
    pub task: &'task Task,
    pub workspace_root: &'task Path,
}

impl<'task> OutputHydrater<'task> {
    pub async fn hydrate(&self, hash: &str, from: HydrateFrom) -> miette::Result<()> {
        // Only hydrate when the hash is different from the previous build,
        // as we can assume the outputs from the previous build still exist?
        if hash.is_empty() || matches!(from, HydrateFrom::PreviousOutput) {
            return Ok(());
        }

        let archive_file = self.cache_engine.hash.get_archive_path(hash);

        if self.cache_engine.get_mode().is_readable() {
            // Attempt to download from remote cache to `.moon/outputs/<hash>`
            if !archive_file.exists() {
                self.download_from_remote_storage(&archive_file, hash)
                    .await?;
            }

            // Otherwise hydrate the cached archive into the task's outputs
            if archive_file.exists() {
                self.unpack_local_archive(&archive_file)?;
            }
        }

        Ok(())
    }

    pub fn unpack_local_archive(&self, archive_file: &Path) -> miette::Result<bool> {
        // If cache disabled or archive doesn't exist, do nothing
        if !self.cache_engine.get_mode().is_readable() || !archive_file.exists() {
            return Ok(false);
        }

        // Create the archiver instance based on task outputs
        let mut archive = Archiver::new(&self.workspace_root, &archive_file);

        for output_file in &self.task.output_files {
            archive.add_source_file(output_file.as_str(), None);
        }

        for output_glob in &self.task.output_globs {
            archive.add_source_glob(output_glob.as_str());
        }

        // Unpack the archive and handle log files
        let state_dir = self.cache_engine.state.get_target_dir(&self.task.target);
        let stdout_log = self.workspace_root.join("stdout.log");
        let stderr_log = self.workspace_root.join("stderr.log");

        match archive.unpack(TarUnpacker::new_gz) {
            Ok(_) => {
                // Old archives place the log files in the root of the
                // archive/workspace, so move them manually
                if stdout_log.exists() {
                    fs::rename(stdout_log, state_dir.join("stdout.log"))?;
                }

                if stderr_log.exists() {
                    fs::rename(stderr_log, state_dir.join("stderr.log"))?;
                }
            }
            Err(error) => {
                warn!(
                    target = self.task.target.as_str(),
                    "Failed to hydrate task outputs from cache: {}",
                    color::muted_light(error.to_string()),
                );

                // Delete target outputs to ensure a clean slate
                for output in &self.task.output_files {
                    fs::remove_file(output.to_logical_path(&self.workspace_root))?;
                }

                // And delete workspace root log files
                fs::remove_file(stdout_log)?;
                fs::remove_file(stderr_log)?;
            }
        }

        Ok(true)
    }

    pub async fn download_from_remote_storage(
        &self,
        _archive_file: &Path,
        _hash: &str,
    ) -> miette::Result<()> {
        Ok(())
    }
}
